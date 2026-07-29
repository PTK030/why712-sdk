//! Transport-neutral chain snapshot acquisition.

use alloy_primitives::{Address, Bytes, U256};
use serde::Serialize;

use crate::{
    DomainSeparator, Erc1271Input, Erc1271Probe, Error, OnchainDomain, Result,
    eip712::domain::{decode_domain_separator, domain_separator_calldata},
    erc1271::{decode_is_valid_signature, encode_is_valid_signature},
};

/// Minimal JSON-RPC port needed to enrich a diagnosis.
#[allow(async_fn_in_trait)] // Both native HTTP and browser fetch are asynchronous transports.
pub trait ChainClient {
    /// Executes `eth_call` at the latest block.
    async fn eth_call(&self, to: Address, data: Bytes) -> Result<Bytes>;

    /// Reads `eth_chainId`.
    async fn eth_chain_id(&self) -> Result<U256>;

    /// Reads runtime bytecode with `eth_getCode` at the latest block.
    async fn eth_get_code(&self, address: Address) -> Result<Bytes>;
}

/// A failed optional probe retained as evidence instead of aborting diagnosis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeFailure {
    /// Solidity probe that failed.
    pub method: &'static str,
    /// Transport, revert, or ABI decoding detail.
    pub reason: String,
}

/// Best on-chain domain information available from the contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "source", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DomainProbe {
    /// ERC-5267 metadata was decoded, with a separator when the legacy getter also succeeded.
    Erc5267 {
        /// Structured domain fields.
        domain: OnchainDomain,
        /// Opaque separator returned by `DOMAIN_SEPARATOR()`, when available.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<DomainSeparator>,
    },
    /// Only the opaque `DOMAIN_SEPARATOR()` getter was available.
    SeparatorOnly {
        /// Separator returned by the contract.
        separator: DomainSeparator,
    },
    /// Neither standard probe produced usable data.
    Unavailable,
}

/// Plain chain observations consumed by deterministic diagnosis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnchainSnapshot {
    /// Chain ID returned by the endpoint.
    pub chain_id: U256,
    /// Contract being diagnosed.
    pub contract: Address,
    /// Runtime bytecode at [`Self::contract`].
    pub contract_code: Bytes,
    /// Runtime bytecode at the expected signer, when one was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer_code: Option<Bytes>,
    /// ERC-1271 result when the expected signer is a contract and signature input was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub erc1271: Option<Erc1271Probe>,
    /// Structured or opaque on-chain domain evidence.
    pub domain: DomainProbe,
    /// Optional probe failures, in attempted order.
    pub failures: Vec<ProbeFailure>,
}

impl OnchainSnapshot {
    /// Whether the expected signer has deployed runtime bytecode.
    #[must_use]
    pub fn signer_is_contract(&self) -> bool {
        self.signer_code
            .as_ref()
            .is_some_and(|code| !code.is_empty())
    }
}

/// Acquires all reproducible on-chain inputs, preserving optional probe failures.
///
/// `eip712Domain()` is attempted first. `DOMAIN_SEPARATOR()` is always attempted next so an
/// ERC-5267 response can be checked for self-consistency; it also serves as the legacy fallback.
///
/// # Errors
///
/// Returns an error only when the essential chain ID or code queries fail. Callers should catch
/// that error and continue with offline diagnosis.
pub async fn acquire_onchain_snapshot<C: ChainClient>(
    client: &C,
    contract: Address,
    expected_signer: Option<Address>,
    erc1271_input: Option<Erc1271Input>,
) -> Result<OnchainSnapshot> {
    let chain_id = client.eth_chain_id().await?;
    let contract_code = client.eth_get_code(contract).await?;
    let signer_code = match expected_signer {
        Some(signer) => Some(client.eth_get_code(signer).await?),
        None => None,
    };
    let mut failures = Vec::new();
    let domain = match client
        .eth_call(contract, Bytes::from(OnchainDomain::calldata()))
        .await
        .and_then(|data| OnchainDomain::decode(&data))
    {
        Ok(domain) => {
            let separator = optional_separator(client, contract, &mut failures).await;
            DomainProbe::Erc5267 { domain, separator }
        }
        Err(error) => {
            failures.push(probe_failure("eip712Domain()", &error));
            match optional_separator(client, contract, &mut failures).await {
                Some(separator) => DomainProbe::SeparatorOnly { separator },
                None => DomainProbe::Unavailable,
            }
        }
    };
    let erc1271 = match (expected_signer, signer_code.as_ref(), erc1271_input) {
        (Some(signer), Some(code), Some(input)) if !code.is_empty() => {
            Some(probe_erc1271(client, signer, input).await)
        }
        _ => None,
    };

    Ok(OnchainSnapshot {
        chain_id,
        contract,
        contract_code,
        signer_code,
        erc1271,
        domain,
        failures,
    })
}

async fn probe_erc1271<C: ChainClient>(
    client: &C,
    signer: Address,
    input: Erc1271Input,
) -> Erc1271Probe {
    let calldata = encode_is_valid_signature(input.hash, input.signature);
    match client.eth_call(signer, Bytes::from(calldata)).await {
        Ok(data) => {
            decode_is_valid_signature(&data).unwrap_or_else(|error| Erc1271Probe::Unavailable {
                reason: error.to_string(),
            })
        }
        Err(error) => Erc1271Probe::Unavailable {
            reason: error.to_string(),
        },
    }
}

async fn optional_separator<C: ChainClient>(
    client: &C,
    contract: Address,
    failures: &mut Vec<ProbeFailure>,
) -> Option<DomainSeparator> {
    match client
        .eth_call(contract, Bytes::from(domain_separator_calldata()))
        .await
        .and_then(|data| decode_domain_separator(&data))
    {
        Ok(separator) => Some(separator),
        Err(error) => {
            failures.push(probe_failure("DOMAIN_SEPARATOR()", &error));
            None
        }
    }
}

fn probe_failure(method: &'static str, error: &Error) -> ProbeFailure {
    ProbeFailure {
        method,
        reason: error.to_string(),
    }
}
