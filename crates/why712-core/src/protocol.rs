//! Protocol recognition and transport-neutral verification evidence.

use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Error, Result, TransactionContext};

/// ERC-6492 suffix defined by the standard.
pub const ERC6492_DETECTION_SUFFIX: [u8; 32] = [
    0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92,
    0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92, 0x64, 0x92,
];

/// Origin of the normalized diagnostic input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InputSource {
    /// A plain EIP-712 typed-data document.
    #[default]
    TypedData,
    /// An `eth_signTypedData_v4` JSON-RPC request.
    WalletRequest,
    /// A recognized verification transaction.
    Transaction,
}

/// One normalized diagnostic input with its original transport representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticInput {
    /// EIP-712 document consumed by the hashing engine.
    pub typed_data_json: String,
    /// Exact JSON supplied by the caller before normalization.
    pub original_json: String,
    /// Transport shape recognized at the boundary.
    pub source: InputSource,
}

/// Signature protocol recognized without guessing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DetectedStandard {
    /// ERC-20 permit.
    Eip2612,
    /// Permit2 ordered allowance.
    Permit2Allowance,
    /// Permit2 unordered signature transfer.
    Permit2SignatureTransfer,
    /// Deployed contract signer.
    Erc1271,
    /// Counterfactual contract signer.
    Erc6492,
}

/// Result of a read-only verification call.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SimulationResult {
    /// The call completed without reverting.
    Succeeded,
    /// The call reverted.
    Reverted {
        /// RPC or decoded revert detail.
        reason: String,
    },
    /// The call could not be executed.
    Unavailable {
        /// Transport or historical-state detail.
        reason: String,
    },
}

/// Result returned by a bounded ERC-6492-compatible validation call.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Erc6492Probe {
    /// The validator accepted the signature.
    Valid,
    /// The validator rejected the signature.
    Invalid,
    /// Validation could not complete.
    Unavailable {
        /// RPC, gas, decoding, or execution detail.
        reason: String,
    },
}

/// Returns whether a hex signature carries the ERC-6492 suffix.
#[must_use]
pub fn is_erc6492_signature(signature: &str) -> bool {
    let value = signature.strip_prefix("0x").unwrap_or(signature);
    let suffix = hex::encode(ERC6492_DETECTION_SUFFIX);
    value.len() > suffix.len() && value.to_ascii_lowercase().ends_with(&suffix)
}

/// EIP-2612 values captured at one block.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Eip2612Snapshot {
    /// Signed owner.
    pub owner: Address,
    /// Nonce in the message.
    pub signed_nonce: U256,
    /// Nonce returned by the token.
    pub onchain_nonce: Option<U256>,
    /// Failure returned while reading the nonce.
    pub state_failure: Option<String>,
    /// Signed deadline.
    pub deadline: U256,
    /// Observed block time.
    pub block_timestamp: Option<u64>,
    /// Read-only permit result.
    pub simulation: Option<SimulationResult>,
}

/// Permit2 allowance values captured at one block.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Permit2AllowanceItemSnapshot {
    /// Nonce in the Permit2 details.
    pub signed_nonce: U256,
    /// Nonce returned by Permit2.
    pub onchain_nonce: Option<U256>,
    /// Failure returned while reading the allowance.
    pub state_failure: Option<String>,
    /// Signed allowance expiration.
    pub expiration: U256,
}

/// Permit2 ordered allowance values captured at one block.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Permit2AllowanceSnapshot {
    /// One state comparison for every signed token detail.
    pub allowances: Vec<Permit2AllowanceItemSnapshot>,
    /// Signed signature deadline.
    pub signature_deadline: U256,
    /// Observed block time.
    pub block_timestamp: Option<u64>,
    /// Read-only call result.
    pub simulation: Option<SimulationResult>,
}

/// Permit2 unordered-nonce values captured at one block.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Permit2SignatureTransferSnapshot {
    /// Signed unordered nonce.
    pub nonce: U256,
    /// Bitmap word containing the nonce bit.
    pub nonce_bitmap: Option<U256>,
    /// Failure returned while reading the bitmap.
    pub state_failure: Option<String>,
    /// Signed deadline.
    pub signature_deadline: U256,
    /// Observed block time.
    pub block_timestamp: Option<u64>,
    /// Read-only call result.
    pub simulation: Option<SimulationResult>,
}

/// Protocol-specific evidence consumed by pure diagnosis.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "standard", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtocolSnapshot {
    /// EIP-2612 state.
    Eip2612(Eip2612Snapshot),
    /// Permit2 ordered allowance state.
    Permit2Allowance(Permit2AllowanceSnapshot),
    /// Permit2 unordered nonce state.
    Permit2SignatureTransfer(Permit2SignatureTransferSnapshot),
}

/// Parsed protocol values required for state acquisition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolRequest {
    /// Standard ERC-2612 permit.
    Eip2612 {
        /// Token owner.
        owner: Address,
        /// Approved spender.
        spender: Address,
        /// Approved amount.
        value: U256,
        /// Signed nonce.
        nonce: U256,
        /// Signed deadline.
        deadline: U256,
    },
    /// Single-token Permit2 allowance.
    Permit2Allowance {
        /// Permit owner, supplied as the expected signer.
        owner: Address,
        /// Spender address.
        spender: Address,
        /// Signed token details.
        details: Vec<Permit2AllowanceRequest>,
        /// Signature deadline.
        signature_deadline: U256,
    },
    /// Permit2 signature transfer.
    Permit2SignatureTransfer {
        /// Permit owner, supplied as the expected signer.
        owner: Address,
        /// Signed unordered nonce.
        nonce: U256,
        /// Signature deadline.
        signature_deadline: U256,
    },
}

/// One signed Permit2 ordered allowance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permit2AllowanceRequest {
    /// Token address.
    pub token: Address,
    /// Signed nonce.
    pub nonce: U256,
    /// Signed expiration.
    pub expiration: U256,
}

/// Context shown in report schema v2.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticContext {
    /// Normalized input origin.
    pub input_source: InputSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Strictly detected standard.
    pub detected_standard: Option<DetectedStandard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// RPC chain ID.
    pub chain_id: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// State block number.
    pub block_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// State block timestamp.
    pub block_timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Protocol-specific state.
    pub protocol: Option<ProtocolSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Decoded ERC-6492 wrapper sizes and factory/prepare target.
    pub erc6492_wrapper: Option<Erc6492WrapperContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Transaction hash, state block, selector, and recognized direct operation.
    pub transaction: Option<TransactionContext>,
}

/// Safe report context decoded from an ERC-6492 wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Erc6492WrapperContext {
    /// Factory for an undeployed account or prepare target for a deployed account.
    pub target: Address,
    /// Byte length of factory or prepare calldata.
    pub calldata_length: usize,
    /// Byte length of the wrapped ERC-1271 signature.
    pub inner_signature_length: usize,
}

/// Fully decoded ERC-6492 wrapper used by validation adapters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Erc6492Wrapper {
    /// Factory for an undeployed account or prepare target for a deployed account.
    pub target: Address,
    /// Factory or prepare calldata.
    pub calldata: Bytes,
    /// Original ERC-1271 signature.
    pub inner_signature: Bytes,
}

impl Erc6492Wrapper {
    /// Returns the privacy-safe subset included in report context.
    #[must_use]
    pub fn context(&self) -> Erc6492WrapperContext {
        Erc6492WrapperContext {
            target: self.target,
            calldata_length: self.calldata.len(),
            inner_signature_length: self.inner_signature.len(),
        }
    }
}

/// Normalizes a plain typed-data document or an `eth_signTypedData_v4` envelope.
///
/// # Errors
///
/// Returns an error when the JSON-RPC request has no usable typed-data parameter.
pub fn normalize_diagnostic_input(input: &str) -> Result<DiagnosticInput> {
    let value: Value = serde_json::from_str(input).map_err(|source| Error::TypedDataShape {
        reason: source.to_string(),
    })?;
    if value.get("method").and_then(Value::as_str) != Some("eth_signTypedData_v4") {
        return Ok(DiagnosticInput {
            typed_data_json: input.to_owned(),
            original_json: input.to_owned(),
            source: InputSource::TypedData,
        });
    }
    let params = value
        .get("params")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::TypedDataShape {
            reason: "eth_signTypedData_v4 params must be an array".to_owned(),
        })?;
    let typed = params.get(1).ok_or_else(|| Error::TypedDataShape {
        reason: "eth_signTypedData_v4 requires typed data as params[1]".to_owned(),
    })?;
    let normalized = match typed {
        Value::String(json) => json.clone(),
        value => serde_json::to_string(value).map_err(|source| Error::TypedDataShape {
            reason: source.to_string(),
        })?,
    };
    Ok(DiagnosticInput {
        typed_data_json: normalized,
        original_json: input.to_owned(),
        source: InputSource::WalletRequest,
    })
}

/// Compatibility wrapper returning the normalized document and source.
///
/// # Errors
///
/// Returns an error under the same conditions as [`normalize_diagnostic_input`].
pub fn normalize_typed_data(input: &str) -> Result<(String, InputSource)> {
    let normalized = normalize_diagnostic_input(input)?;
    Ok((normalized.typed_data_json, normalized.source))
}

/// Decodes an ERC-6492 `(address,bytes,bytes)` wrapper after checking its suffix.
///
/// # Errors
///
/// Returns a typed-data shape error when the signature is malformed hex, lacks the suffix, or
/// does not contain the canonical ABI tuple.
pub fn decode_erc6492_signature(signature: &str) -> Result<Erc6492Wrapper> {
    if !is_erc6492_signature(signature) {
        return Err(shape("signature does not carry the ERC-6492 suffix"));
    }
    let encoded = signature.strip_prefix("0x").unwrap_or(signature);
    let bytes = hex::decode(encoded).map_err(|error| shape(&format!("signature hex: {error}")))?;
    let body = bytes
        .get(..bytes.len().saturating_sub(ERC6492_DETECTION_SUFFIX.len()))
        .ok_or_else(|| shape("ERC-6492 wrapper is shorter than its suffix"))?;
    let (target, calldata, inner_signature) =
        <(Address, Bytes, Bytes)>::abi_decode(body).map_err(|error| {
            shape(&format!(
                "ERC-6492 wrapper must encode (address,bytes,bytes): {error}"
            ))
        })?;
    Ok(Erc6492Wrapper {
        target,
        calldata,
        inner_signature,
    })
}

/// Recognizes supported typed-data shapes using exact primary type and field names.
#[must_use]
pub fn detect_standard(root: &Value) -> Option<DetectedStandard> {
    let primary = root.get("primaryType")?.as_str()?;
    let message = root.get("message")?.as_object()?;
    match primary {
        "Permit"
            if message.len() == 5
                && ["owner", "spender", "value", "nonce", "deadline"]
                    .iter()
                    .all(|field| message.contains_key(*field))
                && exact_type(
                    root,
                    "Permit",
                    &[
                        ("owner", "address"),
                        ("spender", "address"),
                        ("value", "uint256"),
                        ("nonce", "uint256"),
                        ("deadline", "uint256"),
                    ],
                ) =>
        {
            Some(DetectedStandard::Eip2612)
        }
        "PermitSingle"
            if message.len() == 3
                && message.contains_key("details")
                && message.contains_key("spender")
                && message.contains_key("sigDeadline")
                && exact_type(
                    root,
                    "PermitSingle",
                    &[
                        ("details", "PermitDetails"),
                        ("spender", "address"),
                        ("sigDeadline", "uint256"),
                    ],
                ) =>
        {
            Some(DetectedStandard::Permit2Allowance)
        }
        "PermitBatch"
            if message.len() == 3
                && message.contains_key("details")
                && message.contains_key("spender")
                && message.contains_key("sigDeadline")
                && exact_type(
                    root,
                    "PermitBatch",
                    &[
                        ("details", "PermitDetails[]"),
                        ("spender", "address"),
                        ("sigDeadline", "uint256"),
                    ],
                ) =>
        {
            Some(DetectedStandard::Permit2Allowance)
        }
        "PermitTransferFrom"
            if message.len() == 4
                && message.contains_key("permitted")
                && message.contains_key("spender")
                && message.contains_key("nonce")
                && message.contains_key("deadline")
                && exact_type(
                    root,
                    "PermitTransferFrom",
                    &[
                        ("permitted", "TokenPermissions"),
                        ("spender", "address"),
                        ("nonce", "uint256"),
                        ("deadline", "uint256"),
                    ],
                ) =>
        {
            Some(DetectedStandard::Permit2SignatureTransfer)
        }
        "PermitBatchTransferFrom"
            if message.len() == 4
                && message.contains_key("permitted")
                && message.contains_key("spender")
                && message.contains_key("nonce")
                && message.contains_key("deadline")
                && exact_type(
                    root,
                    "PermitBatchTransferFrom",
                    &[
                        ("permitted", "TokenPermissions[]"),
                        ("spender", "address"),
                        ("nonce", "uint256"),
                        ("deadline", "uint256"),
                    ],
                ) =>
        {
            Some(DetectedStandard::Permit2SignatureTransfer)
        }
        _ => None,
    }
}

fn exact_type(root: &Value, name: &str, expected: &[(&str, &str)]) -> bool {
    let Some(fields) = root
        .get("types")
        .and_then(|types| types.get(name))
        .and_then(Value::as_array)
    else {
        return false;
    };
    fields.len() == expected.len()
        && fields.iter().zip(expected).all(|(field, (name, kind))| {
            field.get("name").and_then(Value::as_str) == Some(*name)
                && field.get("type").and_then(Value::as_str) == Some(*kind)
        })
}

/// Parses the exact supported protocol shapes from typed data.
///
/// # Errors
///
/// Returns an error when a recognized shape contains an invalid address or integer.
pub fn protocol_request(
    root: &Value,
    expected_signer: Option<Address>,
) -> Result<Option<ProtocolRequest>> {
    let Some(standard) = detect_standard(root) else {
        return Ok(None);
    };
    let message = root
        .get("message")
        .and_then(Value::as_object)
        .ok_or_else(|| shape("message must be an object"))?;
    match standard {
        DetectedStandard::Eip2612 => Ok(Some(ProtocolRequest::Eip2612 {
            owner: address_field(message.get("owner"), "message.owner")?,
            spender: address_field(message.get("spender"), "message.spender")?,
            value: uint_field(message.get("value"), "message.value")?,
            nonce: uint_field(message.get("nonce"), "message.nonce")?,
            deadline: uint_field(message.get("deadline"), "message.deadline")?,
        })),
        DetectedStandard::Permit2Allowance => {
            let owner = expected_signer.ok_or_else(|| {
                shape("Permit2 state acquisition requires the expected signer as owner")
            })?;
            let details = match message.get("details") {
                Some(Value::Object(details)) => vec![permit2_detail(details)?],
                Some(Value::Array(details)) => details
                    .iter()
                    .map(|detail| {
                        detail
                            .as_object()
                            .ok_or_else(|| shape("Permit2 batch details must be objects"))
                            .and_then(permit2_detail)
                    })
                    .collect::<Result<Vec<_>>>()?,
                _ => return Err(shape("message.details must be an object or array")),
            };
            Ok(Some(ProtocolRequest::Permit2Allowance {
                owner,
                spender: address_field(message.get("spender"), "message.spender")?,
                details,
                signature_deadline: uint_field(message.get("sigDeadline"), "message.sigDeadline")?,
            }))
        }
        DetectedStandard::Permit2SignatureTransfer => {
            let owner = expected_signer.ok_or_else(|| {
                shape("Permit2 state acquisition requires the expected signer as owner")
            })?;
            Ok(Some(ProtocolRequest::Permit2SignatureTransfer {
                owner,
                nonce: uint_field(message.get("nonce"), "message.nonce")?,
                signature_deadline: uint_field(message.get("deadline"), "message.deadline")?,
            }))
        }
        DetectedStandard::Erc1271 | DetectedStandard::Erc6492 => Ok(None),
    }
}

fn permit2_detail(details: &serde_json::Map<String, Value>) -> Result<Permit2AllowanceRequest> {
    Ok(Permit2AllowanceRequest {
        token: address_field(details.get("token"), "message.details.token")?,
        nonce: uint_field(details.get("nonce"), "message.details.nonce")?,
        expiration: uint_field(details.get("expiration"), "message.details.expiration")?,
    })
}

fn address_field(value: Option<&Value>, field: &'static str) -> Result<Address> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| shape(&format!("{field} must be a hex address")))?
        .parse()
        .map_err(|_| Error::Address { field })
}

fn uint_field(value: Option<&Value>, field: &str) -> Result<U256> {
    let encoded = match value {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        _ => {
            return Err(shape(&format!(
                "{field} must be an integer or decimal string"
            )));
        }
    };
    if let Some(hex) = encoded.strip_prefix("0x") {
        U256::from_str_radix(hex, 16).map_err(|error| shape(&format!("{field}: {error}")))
    } else {
        U256::from_str_radix(&encoded, 10).map_err(|error| shape(&format!("{field}: {error}")))
    }
}

fn shape(reason: &str) -> Error {
    Error::TypedDataShape {
        reason: reason.to_owned(),
    }
}
