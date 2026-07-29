//! Pure ERC-1271 ABI helpers and transport-neutral observations.

use alloy_primitives::{B256, Bytes, FixedBytes};
use alloy_sol_types::{SolCall, sol};
use serde::Serialize;

use crate::{Digest, Error, Result};

sol! {
    function isValidSignature(bytes32 hash, bytes signature) external view returns (bytes4);
}

/// Success value required by ERC-1271.
pub const ERC1271_MAGIC_VALUE: FixedBytes<4> = FixedBytes::new([0x16, 0x26, 0xba, 0x7e]);

/// Digest and opaque signature bytes supplied to a contract signer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Erc1271Input {
    /// EIP-712 digest passed to the signer contract.
    pub hash: B256,
    /// Signature bytes interpreted only by the signer contract.
    pub signature: Bytes,
}

impl Erc1271Input {
    /// Builds contract-signature input without imposing an ECDSA length or layout.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Hex`] when the signature is not valid hexadecimal.
    pub fn from_signature_hex(digest: Digest, signature: &str) -> Result<Self> {
        let encoded = signature.strip_prefix("0x").unwrap_or(signature);
        let signature = hex::decode(encoded).map_err(|source| Error::Hex {
            field: "signature",
            source,
        })?;
        Ok(Self {
            hash: *digest.as_b256(),
            signature: Bytes::from(signature),
        })
    }
}

/// Result of asking a contract signer to validate a digest and opaque signature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Erc1271Probe {
    /// The contract returned the ERC-1271 magic value.
    Valid,
    /// The call succeeded and returned another ABI-valid `bytes4` value.
    Invalid {
        /// Value returned by the contract.
        returned: FixedBytes<4>,
    },
    /// The optional call reverted, failed in transport, or returned malformed ABI data.
    Unavailable {
        /// Failure detail retained as deterministic report evidence.
        reason: String,
    },
}

/// Encodes `isValidSignature(bytes32,bytes)` calldata.
#[must_use]
pub fn encode_is_valid_signature(hash: B256, signature: Bytes) -> Vec<u8> {
    isValidSignatureCall { hash, signature }.abi_encode()
}

/// Decodes the standard ABI return and compares it with the required magic value.
///
/// # Errors
///
/// Returns [`Error::Rpc`] when the response is not a valid ABI encoding of `bytes4`.
pub fn decode_is_valid_signature(data: &[u8]) -> Result<Erc1271Probe> {
    let returned =
        isValidSignatureCall::abi_decode_returns_validate(data).map_err(|source| Error::Rpc {
            method: "isValidSignature(bytes32,bytes)",
            reason: format!("invalid bytes4 return data: {source}"),
        })?;
    Ok(if returned == ERC1271_MAGIC_VALUE {
        Erc1271Probe::Valid
    } else {
        Erc1271Probe::Invalid { returned }
    })
}
