//! Native Alloy-backed JSON-RPC transport.

use alloy::{
    providers::{Provider, RootProvider},
    rpc::types::TransactionRequest,
};
use alloy_primitives::{Address, Bytes, U256};

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
}

fn rpc_error(method: &'static str, source: impl core::fmt::Display) -> Error {
    Error::Rpc {
        method,
        reason: source.to_string(),
    }
}
