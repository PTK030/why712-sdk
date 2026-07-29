//! Diagnose why an EIP-712 signature does not verify.
//!
//! The crate answers one question: given some typed data, a signature, and optionally the
//! contract that rejected it, *what specifically is wrong?*
//!
//! Two properties shape the design:
//!
//! - **The core is pure and synchronous.** No network, no clock, no randomness. Anything read
//!   from a chain enters as a plain snapshot value that the caller populates. That is what lets
//!   the same code run in a CLI and in a browser, and what makes every diagnosis reproducible
//!   from its inputs alone.
//! - **We do not implement EIP-712 hashing ourselves.** For a tool whose purpose is telling
//!   people their hashing is wrong, a bug in its own hashing would be the worst possible
//!   failure. `alloy-dyn-abi` is the engine; this crate exposes the intermediates it computes
//!   and reasons about them.

pub mod chain;
pub mod diagnose;
pub mod eip712;
pub mod erc1271;
pub mod error;
pub mod input;
pub mod report;
pub mod signature;

#[cfg(feature = "rpc")]
pub mod rpc;

pub use chain::{
    ChainClient, DomainProbe, OnchainSnapshot, ProbeFailure, acquire_onchain_snapshot,
};
pub use diagnose::diagnose_offline;
pub use eip712::{
    Candidate, ComputedValues, Digest, DomainField, DomainFields, DomainSeparator,
    HypothesisContext, HypothesisHit, HypothesisKind, MAX_CANDIDATES, OnchainDomain, StructHash,
    TypeHash, TypedDataDocument, generate_candidates, search,
};
pub use erc1271::{
    ERC1271_MAGIC_VALUE, Erc1271Input, Erc1271Probe, decode_is_valid_signature,
    encode_is_valid_signature,
};
pub use error::{Error, Result};
pub use input::DiagnoseRequest;
pub use report::{
    Confidence, Finding, FindingCode, REPORT_SCHEMA_VERSION, Report, Severity, Verdict,
    render_markdown,
};
pub use signature::{ParsedSignature, RecoveryEncoding, SignatureFormat};
