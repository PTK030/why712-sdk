//! Browser JSON-RPC transport over `window.fetch`.

use alloy_primitives::{Address, Bytes, U256};
use serde::Deserialize;
use serde_json::{Value, json};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, Response};
use why712_core::{ChainClient, Error, Result};

pub(crate) struct FetchChainClient<'a> {
    endpoint: &'a str,
}

impl<'a> FetchChainClient<'a> {
    pub(crate) const fn new(endpoint: &'a str) -> Self {
        Self { endpoint }
    }

    async fn request(&self, method: &'static str, params: Value) -> Result<Value> {
        let body = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }))
        .map_err(|source| rpc_error(method, source))?;
        let headers = Headers::new().map_err(|source| browser_error(method, &source))?;
        headers
            .set("content-type", "application/json")
            .map_err(|source| browser_error(method, &source))?;
        let init = RequestInit::new();
        init.set_method("POST");
        init.set_headers_headers(&headers);
        init.set_body(&JsValue::from_str(&body));
        let request = Request::new_with_str_and_init(self.endpoint, &init)
            .map_err(|source| browser_error(method, &source))?;
        let window = web_sys::window().ok_or_else(|| Error::Rpc {
            method,
            reason: "window is unavailable".to_owned(),
        })?;
        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|source| cors_error(method, &source))?
            .dyn_into::<Response>()
            .map_err(|source| browser_error(method, &source))?;
        if !response.ok() {
            return Err(Error::Rpc {
                method,
                reason: format!("HTTP {} {}", response.status(), response.status_text()),
            });
        }
        let text = JsFuture::from(
            response
                .text()
                .map_err(|source| browser_error(method, &source))?,
        )
        .await
        .map_err(|source| browser_error(method, &source))?
        .as_string()
        .ok_or_else(|| Error::Rpc {
            method,
            reason: "response body is not text".to_owned(),
        })?;
        let envelope: RpcEnvelope =
            serde_json::from_str(&text).map_err(|source| rpc_error(method, source))?;
        match envelope {
            RpcEnvelope::Success { result } => Ok(result),
            RpcEnvelope::Failure { error } => Err(Error::Rpc {
                method,
                reason: format!("JSON-RPC {}: {}", error.code, error.message),
            }),
        }
    }
}

impl ChainClient for FetchChainClient<'_> {
    async fn eth_call(&self, to: Address, data: Bytes) -> Result<Bytes> {
        let value = self
            .request(
                "eth_call",
                json!([
                    {
                        "to": to.to_string(),
                        "data": format!("0x{}", alloy_primitives::hex::encode(data)),
                    },
                    "latest"
                ]),
            )
            .await?;
        parse_bytes("eth_call", &value)
    }

    async fn eth_chain_id(&self) -> Result<U256> {
        let value = self.request("eth_chainId", json!([])).await?;
        let encoded = string_result("eth_chainId", &value)?;
        U256::from_str_radix(encoded.trim_start_matches("0x"), 16)
            .map_err(|source| rpc_error("eth_chainId", source))
    }

    async fn eth_get_code(&self, address: Address) -> Result<Bytes> {
        let value = self
            .request("eth_getCode", json!([address.to_string(), "latest"]))
            .await?;
        parse_bytes("eth_getCode", &value)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RpcEnvelope {
    Success { result: Value },
    Failure { error: RpcResponseError },
}

#[derive(Debug, Deserialize)]
struct RpcResponseError {
    code: i64,
    message: String,
}

fn parse_bytes(method: &'static str, value: &Value) -> Result<Bytes> {
    let encoded = string_result(method, value)?;
    alloy_primitives::hex::decode(encoded.trim_start_matches("0x"))
        .map(Bytes::from)
        .map_err(|source| rpc_error(method, source))
}

fn string_result<'a>(method: &'static str, value: &'a Value) -> Result<&'a str> {
    value.as_str().ok_or_else(|| Error::Rpc {
        method,
        reason: "result is not a hex string".to_owned(),
    })
}

fn rpc_error(method: &'static str, source: impl core::fmt::Display) -> Error {
    Error::Rpc {
        method,
        reason: source.to_string(),
    }
}

fn browser_error(method: &'static str, source: &JsValue) -> Error {
    Error::Rpc {
        method,
        reason: js_value_message(source),
    }
}

fn cors_error(method: &'static str, source: &JsValue) -> Error {
    Error::Rpc {
        method,
        reason: format!(
            "browser fetch failed; this RPC may block browser requests (CORS). Use another \
             endpoint, clear the RPC field for offline mode, or use the CLI: {}",
            js_value_message(source)
        ),
    }
}

fn js_value_message(value: &JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "JavaScript rejected the operation without a message".to_owned())
}
