//! JavaScript JSON-RPC transport over `globalThis.fetch`.

use alloy_primitives::{Address, B256, Bytes, U256};
use js_sys::{Function, Reflect};
use serde::Deserialize;
use serde_json::{Value, json};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, Response};
use why712_core::{ChainClient, ChainTransaction, Error, Result};

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
        let headers = Headers::new().map_err(|source| javascript_error(method, &source))?;
        headers
            .set("content-type", "application/json")
            .map_err(|source| javascript_error(method, &source))?;
        let init = RequestInit::new();
        init.set_method("POST");
        init.set_headers_headers(&headers);
        init.set_body(&JsValue::from_str(&body));
        let request = Request::new_with_str_and_init(self.endpoint, &init)
            .map_err(|source| javascript_error(method, &source))?;
        let global = js_sys::global();
        let fetch = Reflect::get(&global, &JsValue::from_str("fetch"))
            .map_err(|source| javascript_error(method, &source))?
            .dyn_into::<Function>()
            .map_err(|source| javascript_error(method, &source))?;
        let promise = fetch
            .call1(&global, request.as_ref())
            .map_err(|source| fetch_error(method, &source))?
            .dyn_into::<js_sys::Promise>()
            .map_err(|source| javascript_error(method, &source))?;
        let response = JsFuture::from(promise)
            .await
            .map_err(|source| fetch_error(method, &source))?
            .dyn_into::<Response>()
            .map_err(|source| javascript_error(method, &source))?;
        if !response.ok() {
            return Err(Error::Rpc {
                method,
                reason: format!("HTTP {} {}", response.status(), response.status_text()),
            });
        }
        let text = JsFuture::from(
            response
                .text()
                .map_err(|source| javascript_error(method, &source))?,
        )
        .await
        .map_err(|source| javascript_error(method, &source))?
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

    async fn eth_call_at(&self, to: Address, data: Bytes, block: Option<u64>) -> Result<Bytes> {
        let Some(block) = block else {
            return self.eth_call(to, data).await;
        };
        let value = self
            .request(
                "eth_call",
                json!([
                    {
                        "to": to.to_string(),
                        "data": format!("0x{}", alloy_primitives::hex::encode(data)),
                    },
                    format!("0x{block:x}")
                ]),
            )
            .await?;
        parse_bytes("eth_call", &value)
    }

    async fn eth_call_from_at(
        &self,
        to: Address,
        from: Address,
        data: Bytes,
        block: Option<u64>,
    ) -> Result<Bytes> {
        let block = block.map_or_else(|| "latest".to_owned(), |value| format!("0x{value:x}"));
        let value = self
            .request(
                "eth_call",
                json!([
                    {
                        "to": to.to_string(),
                        "from": from.to_string(),
                        "data": format!("0x{}", alloy_primitives::hex::encode(data)),
                    },
                    block
                ]),
            )
            .await?;
        parse_bytes("eth_call", &value)
    }

    async fn eth_call_limited(&self, to: Address, data: Bytes, gas: u64) -> Result<Bytes> {
        let value = self
            .request(
                "eth_call",
                json!([
                    {
                        "to": to.to_string(),
                        "data": format!("0x{}", alloy_primitives::hex::encode(data)),
                        "gas": format!("0x{gas:x}"),
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

    async fn eth_get_code_at(&self, address: Address, block: Option<u64>) -> Result<Bytes> {
        let block = block.map_or_else(|| "latest".to_owned(), |value| format!("0x{value:x}"));
        let value = self
            .request("eth_getCode", json!([address.to_string(), block]))
            .await?;
        parse_bytes("eth_getCode", &value)
    }

    async fn eth_get_transaction_by_hash(&self, hash: B256) -> Result<ChainTransaction> {
        let value = self
            .request("eth_getTransactionByHash", json!([hash.to_string()]))
            .await?;
        parse_transaction(hash, &value)
    }

    async fn eth_block_number(&self) -> Result<u64> {
        let value = self.request("eth_blockNumber", json!([])).await?;
        let encoded = string_result("eth_blockNumber", &value)?;
        u64::from_str_radix(encoded.trim_start_matches("0x"), 16)
            .map_err(|source| rpc_error("eth_blockNumber", source))
    }

    async fn eth_block_timestamp(&self, block_number: u64) -> Result<u64> {
        let value = self
            .request(
                "eth_getBlockByNumber",
                json!([format!("0x{block_number:x}"), false]),
            )
            .await?;
        let encoded = value
            .get("timestamp")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Rpc {
                method: "eth_getBlockByNumber",
                reason: "result has no timestamp".to_owned(),
            })?;
        u64::from_str_radix(encoded.trim_start_matches("0x"), 16)
            .map_err(|source| rpc_error("eth_getBlockByNumber", source))
    }
}

fn parse_transaction(hash: B256, value: &Value) -> Result<ChainTransaction> {
    if value.is_null() {
        return Err(Error::Rpc {
            method: "eth_getTransactionByHash",
            reason: format!("transaction {hash} was not found"),
        });
    }
    let field = |name: &'static str| {
        value
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Rpc {
                method: "eth_getTransactionByHash",
                reason: format!("result has no {name}"),
            })
    };
    let to = field("to")?.parse().map_err(|_| Error::Rpc {
        method: "eth_getTransactionByHash",
        reason: "result has an invalid to address".to_owned(),
    })?;
    let from = field("from")?.parse().map_err(|_| Error::Rpc {
        method: "eth_getTransactionByHash",
        reason: "result has an invalid from address".to_owned(),
    })?;
    let input = alloy_primitives::hex::decode(field("input")?.trim_start_matches("0x"))
        .map(Bytes::from)
        .map_err(|source| rpc_error("eth_getTransactionByHash", source))?;
    let block_number = value
        .get("blockNumber")
        .and_then(Value::as_str)
        .map(|encoded| u64::from_str_radix(encoded.trim_start_matches("0x"), 16))
        .transpose()
        .map_err(|source| rpc_error("eth_getTransactionByHash", source))?;
    Ok(ChainTransaction {
        original_json: value.to_string(),
        hash,
        to,
        from,
        input,
        block_number,
    })
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

fn javascript_error(method: &'static str, source: &JsValue) -> Error {
    Error::Rpc {
        method,
        reason: js_value_message(source),
    }
}

fn fetch_error(method: &'static str, source: &JsValue) -> Error {
    Error::Rpc {
        method,
        reason: format!(
            "JavaScript fetch failed; the RPC may be unreachable or reject browser requests \
             (CORS). Use another endpoint or omit rpcUrl for offline diagnosis: {}",
            js_value_message(source)
        ),
    }
}

fn js_value_message(value: &JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "JavaScript rejected the operation without a message".to_owned())
}
