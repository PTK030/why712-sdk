//! Error type for inputs the crate cannot make sense of.
//!
//! An [`Error`] means *we could not perform a diagnosis* — malformed JSON, a signature that is
//! not a signature, an RPC that did not answer. It never means *the signature is invalid*.
//! That is a finding, not an error, and it travels in the report so the caller can explain it.
//! Collapsing the two would make the tool report "error" for exactly the case it exists to
//! diagnose.

use thiserror::Error as ThisError;

/// Result alias for fallible operations in this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Something about the input made a diagnosis impossible.
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum Error {
    /// The typed-data payload was not valid JSON.
    #[error("typed data is not valid JSON: {source}")]
    TypedDataJson {
        /// The underlying parse failure.
        #[source]
        source: serde_json::Error,
    },

    /// The JSON parsed, but is not a well-formed EIP-712 typed-data document.
    #[error("typed data is not a valid EIP-712 document: {reason}")]
    TypedDataShape {
        /// What specifically is wrong, in terms the user can act on.
        reason: String,
    },

    /// A hex string could not be decoded.
    #[error("`{field}` is not valid hex: {source}")]
    Hex {
        /// Which input field failed to decode.
        field: &'static str,
        /// The underlying decode failure.
        #[source]
        source: hex::FromHexError,
    },

    /// A signature was not a length this crate recognizes.
    ///
    /// Carries the length so the caller can distinguish a truncated ECDSA signature from a
    /// smart-account signature, which is a different diagnosis entirely.
    #[error("signature is {len} bytes; expected 65 (r,s,v) or 64 (ERC-2098 compact)")]
    SignatureLength {
        /// The length actually supplied, in bytes.
        len: usize,
    },

    /// An address was not 20 bytes of hex.
    #[error("`{field}` is not a valid 20-byte address")]
    Address {
        /// Which input field failed to parse.
        field: &'static str,
    },

    /// The chain did not answer, or answered with something undecodable.
    #[error("RPC call `{method}` failed: {reason}")]
    Rpc {
        /// The JSON-RPC method that failed.
        method: &'static str,
        /// What went wrong, verbatim where possible.
        reason: String,
    },
}
