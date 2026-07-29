//! Native Alloy-backed JSON-RPC transport.

use alloy::{
    providers::{Provider, RootProvider},
    rpc::types::TransactionRequest,
};
use alloy_primitives::{Address, Bytes, U256};
use serde_json::{Value, json};

use crate::{ChainClient, Error, Result};

/// Native HTTP implementation of the minimal chain client.
#[derive(Clone, Debug)]
pub struct AlloyChainClient {
    provider: RootProvider,
}

impl AlloyChainClient {
    /// Connects an HTTP endpoint without issuing a request.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Rpc`] when the endpoint string is not a supported connection URL.
    pub async fn connect(endpoint: &str) -> Result<Self> {
        let provider = RootProvider::connect(endpoint)
            .await
            .map_err(|source| rpc_error("connect", source))?;
        Ok(Self { provider })
    }
}

impl ChainClient for AlloyChainClient {
    async fn eth_call(&self, to: Address, data: Bytes) -> Result<Bytes> {
        let request = TransactionRequest::default().to(to).input(data.into());
        self.provider
            .call(request)
            .await
            .map_err(|source| rpc_error("eth_call", source))
    }

    async fn eth_call_at(&self, to: Address, data: Bytes, block: Option<u64>) -> Result<Bytes> {
        let Some(block) = block else {
            return self.eth_call(to, data).await;
        };
        let value: String = self
            .provider
            .raw_request(
                "eth_call".into(),
                (
                    json!({
                        "to": to.to_string(),
                        "data": format!("0x{}", alloy_primitives::hex::encode(data)),
                    }),
                    format!("0x{block:x}"),
                ),
            )
            .await
            .map_err(|source| rpc_error("eth_call", source))?;
        alloy_primitives::hex::decode(value.trim_start_matches("0x"))
            .map(Bytes::from)
            .map_err(|source| rpc_error("eth_call", source))
    }

    async fn eth_call_limited(&self, to: Address, data: Bytes, gas: u64) -> Result<Bytes> {
        let request = TransactionRequest::default()
            .to(to)
            .gas_limit(gas)
            .input(data.into());
        self.provider
            .call(request)
            .await
            .map_err(|source| rpc_error("eth_call", source))
    }

    async fn eth_chain_id(&self) -> Result<U256> {
        self.provider
            .get_chain_id()
            .await
            .map(U256::from)
            .map_err(|source| rpc_error("eth_chainId", source))
    }

    async fn eth_get_code(&self, address: Address) -> Result<Bytes> {
        self.provider
            .get_code_at(address)
            .await
            .map_err(|source| rpc_error("eth_getCode", source))
    }

    async fn eth_block_number(&self) -> Result<u64> {
        let value: String = self
            .provider
            .raw_request("eth_blockNumber".into(), ())
            .await
            .map_err(|source| rpc_error("eth_blockNumber", source))?;
        u64::from_str_radix(value.trim_start_matches("0x"), 16)
            .map_err(|source| rpc_error("eth_blockNumber", source))
    }

    async fn eth_block_timestamp(&self, block_number: u64) -> Result<u64> {
        let value: Value = self
            .provider
            .raw_request(
                "eth_getBlockByNumber".into(),
                (format!("0x{block_number:x}"), false),
            )
            .await
            .map_err(|source| rpc_error("eth_getBlockByNumber", source))?;
        let timestamp = value
            .get("timestamp")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Rpc {
                method: "eth_getBlockByNumber",
                reason: format!("result has no timestamp: {}", json!(value)),
            })?;
        u64::from_str_radix(timestamp.trim_start_matches("0x"), 16)
            .map_err(|source| rpc_error("eth_getBlockByNumber", source))
    }
}

fn rpc_error(method: &'static str, source: impl core::fmt::Display) -> Error {
    Error::Rpc {
        method,
        reason: source.to_string(),
    }
}
