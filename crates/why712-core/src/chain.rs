//! Transport-neutral chain snapshot acquisition.

use alloy_primitives::{Address, B256, Bytes, U256, keccak256};
use serde::Serialize;

use crate::{
    DomainSeparator, Eip2612Snapshot, Erc1271Input, Erc1271Probe, Erc6492Probe, Error,
    ExecutionCall, OnchainDomain, ParsedSignature, Permit2AllowanceItemSnapshot,
    Permit2AllowanceSnapshot, Permit2SignatureTransferSnapshot, ProtocolRequest, ProtocolSnapshot,
    Result, SimulationResult,
    eip712::domain::{decode_domain_separator, domain_separator_calldata},
    erc1271::{decode_is_valid_signature, encode_is_valid_signature},
};

/// Minimal JSON-RPC port needed to enrich a diagnosis.
#[allow(async_fn_in_trait)] // Both native HTTP and browser fetch are asynchronous transports.
pub trait ChainClient {
    /// Executes `eth_call` at the latest block.
    async fn eth_call(&self, to: Address, data: Bytes) -> Result<Bytes>;

    /// Executes `eth_call` at an explicit block number when supplied.
    async fn eth_call_at(&self, to: Address, data: Bytes, _block: Option<u64>) -> Result<Bytes> {
        self.eth_call(to, data).await
    }

    /// Executes `eth_call` with the original transaction sender at an explicit block.
    async fn eth_call_from_at(
        &self,
        to: Address,
        _from: Address,
        data: Bytes,
        block: Option<u64>,
    ) -> Result<Bytes> {
        self.eth_call_at(to, data, block).await
    }

    /// Executes `eth_call` with an explicit gas ceiling.
    async fn eth_call_limited(&self, to: Address, data: Bytes, _gas: u64) -> Result<Bytes> {
        self.eth_call(to, data).await
    }

    /// Reads `eth_chainId`.
    async fn eth_chain_id(&self) -> Result<U256>;

    /// Reads runtime bytecode with `eth_getCode` at the latest block.
    async fn eth_get_code(&self, address: Address) -> Result<Bytes>;

    /// Reads runtime bytecode at an explicit block number when supplied.
    async fn eth_get_code_at(&self, address: Address, _block: Option<u64>) -> Result<Bytes> {
        self.eth_get_code(address).await
    }

    /// Fetches a transaction required for transaction-hash diagnosis.
    async fn eth_get_transaction_by_hash(&self, _hash: B256) -> Result<ChainTransaction> {
        Err(Error::Rpc {
            method: "eth_getTransactionByHash",
            reason: "transport does not implement transaction lookup".to_owned(),
        })
    }

    /// Reads the latest block number.
    async fn eth_block_number(&self) -> Result<u64> {
        Err(Error::Rpc {
            method: "eth_blockNumber",
            reason: "transport does not implement block lookup".to_owned(),
        })
    }

    /// Reads the timestamp for a block number.
    async fn eth_block_timestamp(&self, _block_number: u64) -> Result<u64> {
        Err(Error::Rpc {
            method: "eth_getBlockByNumber",
            reason: "transport does not implement block lookup".to_owned(),
        })
    }
}

/// Transaction fields required for deterministic direct-call reconstruction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainTransaction {
    /// Original JSON-RPC transaction object retained for reproducibility.
    pub original_json: String,
    /// Transaction hash returned by the endpoint.
    pub hash: B256,
    /// Direct call target.
    pub to: Address,
    /// Transaction sender, which is significant for Permit2.
    pub from: Address,
    /// Complete calldata.
    pub input: Bytes,
    /// Inclusion block, or `None` while pending.
    pub block_number: Option<u64>,
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
    /// ERC-6492 result when a counterfactual wrapper was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub erc6492: Option<Erc6492Probe>,
    /// Structured or opaque on-chain domain evidence.
    pub domain: DomainProbe,
    /// Optional probe failures, in attempted order.
    pub failures: Vec<ProbeFailure>,
    /// Block number used for stateful protocol reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    /// Timestamp of the block used for deadline comparisons.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_timestamp: Option<u64>,
    /// Optional protocol-specific state captured at the same block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<ProtocolSnapshot>,
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
    let mut metadata_failures = Vec::new();
    let (block_number, block_timestamp) = match client.eth_block_number().await {
        Ok(number) => match client.eth_block_timestamp(number).await {
            Ok(timestamp) => (Some(number), Some(timestamp)),
            Err(error) => {
                metadata_failures.push(probe_failure("eth_getBlockByNumber", &error));
                (Some(number), None)
            }
        },
        Err(error) => {
            metadata_failures.push(probe_failure("eth_blockNumber", &error));
            (None, None)
        }
    };
    let mut snapshot = acquire_onchain_snapshot_at(
        client,
        contract,
        expected_signer,
        erc1271_input,
        block_number,
        block_timestamp,
    )
    .await?;
    snapshot.failures.splice(0..0, metadata_failures);
    Ok(snapshot)
}

/// Acquires all chain evidence at a caller-selected state block.
///
/// This is used for transaction diagnosis: the parent block for mined transactions and the
/// current latest block for pending transactions.
///
/// # Errors
///
/// Returns an error when essential chain ID, code, or historical-state reads fail.
pub async fn acquire_onchain_snapshot_at<C: ChainClient>(
    client: &C,
    contract: Address,
    expected_signer: Option<Address>,
    erc1271_input: Option<Erc1271Input>,
    block_number: Option<u64>,
    block_timestamp: Option<u64>,
) -> Result<OnchainSnapshot> {
    let chain_id = client.eth_chain_id().await?;
    let contract_code = client.eth_get_code_at(contract, block_number).await?;
    let signer_code = match expected_signer {
        Some(signer) => Some(client.eth_get_code_at(signer, block_number).await?),
        None => None,
    };
    let mut failures = Vec::new();
    let domain = match client
        .eth_call_at(
            contract,
            Bytes::from(OnchainDomain::calldata()),
            block_number,
        )
        .await
        .and_then(|data| OnchainDomain::decode(&data))
    {
        Ok(domain) => {
            let separator = optional_separator(client, contract, block_number, &mut failures).await;
            DomainProbe::Erc5267 { domain, separator }
        }
        Err(error) => {
            failures.push(probe_failure("eip712Domain()", &error));
            match optional_separator(client, contract, block_number, &mut failures).await {
                Some(separator) => DomainProbe::SeparatorOnly { separator },
                None => DomainProbe::Unavailable,
            }
        }
    };
    let erc1271 = match (expected_signer, signer_code.as_ref(), erc1271_input) {
        (Some(signer), Some(code), Some(input)) if !code.is_empty() => {
            Some(probe_erc1271(client, signer, input, block_number).await)
        }
        _ => None,
    };

    Ok(OnchainSnapshot {
        chain_id,
        contract,
        contract_code,
        signer_code,
        erc1271,
        erc6492: None,
        domain,
        failures,
        block_number,
        block_timestamp,
        protocol: None,
    })
}

/// Acquires state for one strictly recognized signature protocol.
#[allow(clippy::too_many_lines)] // Keeping the protocol fan-out together makes every RPC branch auditable.
pub async fn acquire_protocol_snapshot<C: ChainClient>(
    client: &C,
    contract: Address,
    request: ProtocolRequest,
    signature: Option<&str>,
    execution: Option<&ExecutionCall>,
    block_number: Option<u64>,
    block_timestamp: Option<u64>,
) -> ProtocolSnapshot {
    match request {
        ProtocolRequest::Eip2612 {
            owner,
            spender,
            value,
            nonce,
            deadline,
        } => {
            let nonce_result = read_single_u256(
                client,
                contract,
                static_call("nonces(address)", &[address_word(owner)]),
                block_number,
            )
            .await;
            let generated_simulation = signature.map(|encoded| {
                ParsedSignature::parse_hex(encoded).and_then(|parsed| {
                    let parity =
                        parsed
                            .recovery()
                            .parity()
                            .ok_or_else(|| Error::TypedDataShape {
                                reason: "permit simulation requires a valid recovery byte"
                                    .to_owned(),
                            })?;
                    let words = [
                        address_word(owner),
                        address_word(spender),
                        value.to_be_bytes(),
                        deadline.to_be_bytes(),
                        U256::from(if parity { 28 } else { 27 }).to_be_bytes(),
                        parsed.r().to_be_bytes(),
                        parsed.s().to_be_bytes(),
                    ];
                    Ok(static_call(
                        "permit(address,address,uint256,uint256,uint8,bytes32,bytes32)",
                        &words,
                    ))
                })
            });
            let simulation_call = execution
                .map(|call| Ok((Some(call.from), call.calldata.clone())))
                .or_else(|| generated_simulation.map(|result| result.map(|data| (None, data))));
            let simulation = if let Err(error) = &nonce_result {
                Some(SimulationResult::Unavailable {
                    reason: format!("state read failed before simulation: {error}"),
                })
            } else {
                match simulation_call {
                    Some(Ok((from, calldata))) => Some(
                        match match from {
                            Some(from) => {
                                client
                                    .eth_call_from_at(contract, from, calldata, block_number)
                                    .await
                            }
                            None => client.eth_call_at(contract, calldata, block_number).await,
                        } {
                            Ok(_) => SimulationResult::Succeeded,
                            Err(error) => SimulationResult::Reverted {
                                reason: error.to_string(),
                            },
                        },
                    ),
                    Some(Err(error)) => Some(SimulationResult::Unavailable {
                        reason: error.to_string(),
                    }),
                    None => None,
                }
            };
            ProtocolSnapshot::Eip2612(Eip2612Snapshot {
                owner,
                signed_nonce: nonce,
                onchain_nonce: nonce_result.as_ref().ok().copied(),
                state_failure: nonce_result.err().map(|error| error.to_string()),
                deadline,
                block_timestamp,
                simulation,
            })
        }
        ProtocolRequest::Permit2Allowance {
            owner,
            spender,
            details,
            signature_deadline,
        } => {
            let mut allowances = Vec::with_capacity(details.len());
            for detail in details {
                let calldata = static_call(
                    "allowance(address,address,address)",
                    &[
                        address_word(owner),
                        address_word(detail.token),
                        address_word(spender),
                    ],
                );
                let allowance_result = client
                    .eth_call_at(contract, calldata, block_number)
                    .await
                    .and_then(|data| {
                        word(&data, 2).ok_or_else(|| Error::Rpc {
                            method: "eth_call",
                            reason: "Permit2 allowance result is shorter than three ABI words"
                                .to_owned(),
                        })
                    });
                allowances.push(Permit2AllowanceItemSnapshot {
                    signed_nonce: detail.nonce,
                    onchain_nonce: allowance_result.as_ref().ok().copied(),
                    state_failure: allowance_result.err().map(|error| error.to_string()),
                    expiration: detail.expiration,
                });
            }
            let state_failure = allowances
                .iter()
                .find_map(|item| item.state_failure.clone());
            ProtocolSnapshot::Permit2Allowance(Permit2AllowanceSnapshot {
                allowances,
                signature_deadline,
                block_timestamp,
                simulation: if let Some(reason) = state_failure {
                    Some(SimulationResult::Unavailable {
                        reason: format!("state read failed before simulation: {reason}"),
                    })
                } else {
                    match execution {
                        Some(call) => Some(
                            match client
                                .eth_call_from_at(
                                    contract,
                                    call.from,
                                    call.calldata.clone(),
                                    block_number,
                                )
                                .await
                            {
                                Ok(_) => SimulationResult::Succeeded,
                                Err(error) => SimulationResult::Reverted {
                                    reason: error.to_string(),
                                },
                            },
                        ),
                        None => signature.map(|_| SimulationResult::Unavailable {
                            reason: "Permit2 simulation requires recognized execution calldata"
                                .to_owned(),
                        }),
                    }
                },
            })
        }
        ProtocolRequest::Permit2SignatureTransfer {
            owner,
            nonce,
            signature_deadline,
        } => {
            let word_position: U256 = nonce >> 8;
            let bitmap_result = read_single_u256(
                client,
                contract,
                static_call(
                    "nonceBitmap(address,uint256)",
                    &[address_word(owner), word_position.to_be_bytes()],
                ),
                block_number,
            )
            .await;
            let simulation = if let Err(error) = &bitmap_result {
                Some(SimulationResult::Unavailable {
                    reason: format!("state read failed before simulation: {error}"),
                })
            } else {
                match execution {
                    Some(call) => Some(
                        match client
                            .eth_call_from_at(
                                contract,
                                call.from,
                                call.calldata.clone(),
                                block_number,
                            )
                            .await
                        {
                            Ok(_) => SimulationResult::Succeeded,
                            Err(error) => SimulationResult::Reverted {
                                reason: error.to_string(),
                            },
                        },
                    ),
                    None => signature.map(|_| SimulationResult::Unavailable {
                        reason:
                            "Permit2 transfer simulation requires recognized execution calldata"
                                .to_owned(),
                    }),
                }
            };
            ProtocolSnapshot::Permit2SignatureTransfer(Permit2SignatureTransferSnapshot {
                nonce,
                nonce_bitmap: bitmap_result.as_ref().ok().copied(),
                state_failure: bitmap_result.err().map(|error| error.to_string()),
                signature_deadline,
                block_timestamp,
                simulation,
            })
        }
    }
}

async fn read_single_u256<C: ChainClient>(
    client: &C,
    contract: Address,
    calldata: Bytes,
    block_number: Option<u64>,
) -> Result<U256> {
    let data = client.eth_call_at(contract, calldata, block_number).await?;
    word(&data, 0).ok_or_else(|| Error::Rpc {
        method: "eth_call",
        reason: "result is shorter than one ABI word".to_owned(),
    })
}

fn static_call(signature: &str, words: &[[u8; 32]]) -> Bytes {
    let mut calldata = Vec::with_capacity(4 + words.len() * 32);
    calldata.extend_from_slice(&keccak256(signature.as_bytes())[..4]);
    for value in words {
        calldata.extend_from_slice(value);
    }
    Bytes::from(calldata)
}

fn address_word(address: Address) -> [u8; 32] {
    let mut word = [0_u8; 32];
    word[12..].copy_from_slice(address.as_slice());
    word
}

fn word(data: &[u8], index: usize) -> Option<U256> {
    let start = index.checked_mul(32)?;
    let end = start.checked_add(32)?;
    data.get(start..end).map(U256::from_be_slice)
}

async fn probe_erc1271<C: ChainClient>(
    client: &C,
    signer: Address,
    input: Erc1271Input,
    block_number: Option<u64>,
) -> Erc1271Probe {
    let calldata = encode_is_valid_signature(input.hash, input.signature);
    match client
        .eth_call_at(signer, Bytes::from(calldata), block_number)
        .await
    {
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
    block_number: Option<u64>,
    failures: &mut Vec<ProbeFailure>,
) -> Option<DomainSeparator> {
    match client
        .eth_call_at(
            contract,
            Bytes::from(domain_separator_calldata()),
            block_number,
        )
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
