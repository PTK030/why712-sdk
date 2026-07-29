//! Stable diagnostic report model shared by the CLI, WASM, and markdown renderer.

mod markdown;

use core::fmt;

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

use crate::{ComputedValues, DiagnosticContext};

pub use markdown::render_markdown;

/// Current version of the serialized report contract.
pub const REPORT_SCHEMA_VERSION: u32 = 2;

/// Overall outcome of a diagnosis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    /// The supplied signature recovers the expected signer.
    Match,
    /// At least one concrete mismatch was identified.
    Mismatch,
    /// The available inputs are insufficient for a definitive answer.
    Inconclusive,
}

/// User-facing importance of a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    /// Supporting context that does not itself invalidate the signature.
    Info,
    /// Non-canonical or risky input that may be rejected by some consumers.
    Warning,
    /// A condition that prevents verification under the supplied inputs.
    Error,
}

/// Strength of the evidence behind a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Confidence {
    /// The supplied signature recovered the expected signer under this exact condition.
    Certain,
    /// A direct deterministic check proves the condition, but not that it caused rejection.
    High,
    /// The input has a known risk pattern without proof that information was already lost.
    Heuristic,
}

/// Stable machine-readable reason for a diagnostic finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingCode {
    /// Local and on-chain domain separators differ.
    DomainSeparatorMismatch,
    /// The signing domain used another chain ID.
    DomainChainIdMismatch,
    /// The signing domain used another name.
    DomainNameMismatch,
    /// The signing domain used another version.
    DomainVersionMismatch,
    /// The supplied domain contains a version that the signer omitted.
    DomainVersionUnexpected,
    /// The signing domain used another verifying contract.
    DomainVerifyingContractMismatch,
    /// The supplied domain omitted a salt used by the signer.
    DomainSaltMissing,
    /// The signing domain used another salt.
    DomainSaltMismatch,
    /// The primary type hash differs from the signed type hash.
    TypehashMismatch,
    /// Reordering declared struct fields reproduces the signature.
    FieldOrderMismatch,
    /// Packed ABI encoding reproduces the signature.
    EncodePackedUsed,
    /// Narrowing or widening an integer type reproduces the signature.
    IntegerWidthMismatch,
    /// A JSON number may already have lost precision in JavaScript.
    MessageNumericPrecisionLoss,
    /// The recovery bit uses `0`/`1` instead of `27`/`28`.
    SigVNotNormalized,
    /// The recovery byte is not a recognized Ethereum encoding.
    SigVInvalid,
    /// The signature uses a malleable high-S scalar.
    SigHighS,
    /// The signature's `r` scalar is zero.
    SigZeroR,
    /// The signature's `s` scalar is zero.
    SigZeroS,
    /// Recovery produced an address other than the expected signer.
    SignerMismatch,
    /// The expected signer has contract code and may require ERC-1271.
    SignerIsContract,
    /// The signer contract accepted the digest and opaque signature through ERC-1271.
    Erc1271SignatureValid,
    /// The signer contract rejected the digest and opaque signature through ERC-1271.
    Erc1271SignatureInvalid,
    /// ERC-1271 validation could not produce a standard result.
    Erc1271CallFailed,
    /// ERC-5267 fields do not reproduce the contract's domain separator.
    OnchainDomainSelfInconsistent,
    /// Records which on-chain domain probe produced usable evidence.
    OnchainDomainSource,
    /// The signature was made over the bare primary struct hash.
    BareStructHashSigned,
    /// The signature omitted the EIP-712 `0x1901` prefix.
    Eip712PrefixMissing,
    /// EIP-191 personal signing reproduces the signature.
    PersonalSignDetected,
    /// A dynamic `string` or `bytes` value was encoded without hashing.
    UnhashedDynamicValue,
    /// The signed EIP-2612 nonce differs from contract state.
    PermitNonceMismatch,
    /// The EIP-2612 deadline is earlier than the observed block time.
    PermitExpired,
    /// A deadline resembles a JavaScript millisecond timestamp.
    DeadlineLooksLikeMilliseconds,
    /// EIP-2612 uses the zero address as owner.
    PermitOwnerZero,
    /// A read-only EIP-2612 permit call reverted.
    PermitCallReverted,
    /// A read-only EIP-2612 permit call succeeded.
    PermitCallSucceeded,
    /// A Permit2 unordered nonce bit is already set.
    Permit2NonceUsed,
    /// A Permit2 allowance nonce differs from contract state.
    Permit2NonceMismatch,
    /// A Permit2 signature deadline has passed.
    Permit2SignatureExpired,
    /// A Permit2 allowance expiration has passed.
    Permit2AllowanceExpired,
    /// A read-only Permit2 call reverted.
    Permit2CallReverted,
    /// A read-only Permit2 call succeeded.
    Permit2CallSucceeded,
    /// The signature uses the ERC-6492 wrapper.
    Erc6492WrapperDetected,
    /// ERC-6492 validation succeeded.
    Erc6492SignatureValid,
    /// ERC-6492 validation returned false.
    Erc6492SignatureInvalid,
    /// ERC-6492 validation could not be completed.
    Erc6492ValidationFailed,
    /// Required protocol state could not be read.
    ProtocolStateUnavailable,
}

impl FindingCode {
    /// Every supported code, used to keep the public catalog complete.
    pub const ALL: [Self; 46] = [
        Self::DomainSeparatorMismatch,
        Self::DomainChainIdMismatch,
        Self::DomainNameMismatch,
        Self::DomainVersionMismatch,
        Self::DomainVersionUnexpected,
        Self::DomainVerifyingContractMismatch,
        Self::DomainSaltMissing,
        Self::DomainSaltMismatch,
        Self::TypehashMismatch,
        Self::FieldOrderMismatch,
        Self::EncodePackedUsed,
        Self::IntegerWidthMismatch,
        Self::MessageNumericPrecisionLoss,
        Self::SigVNotNormalized,
        Self::SigVInvalid,
        Self::SigHighS,
        Self::SigZeroR,
        Self::SigZeroS,
        Self::SignerMismatch,
        Self::SignerIsContract,
        Self::Erc1271SignatureValid,
        Self::Erc1271SignatureInvalid,
        Self::Erc1271CallFailed,
        Self::OnchainDomainSelfInconsistent,
        Self::OnchainDomainSource,
        Self::BareStructHashSigned,
        Self::Eip712PrefixMissing,
        Self::PersonalSignDetected,
        Self::UnhashedDynamicValue,
        Self::PermitNonceMismatch,
        Self::PermitExpired,
        Self::DeadlineLooksLikeMilliseconds,
        Self::PermitOwnerZero,
        Self::PermitCallReverted,
        Self::PermitCallSucceeded,
        Self::Permit2NonceUsed,
        Self::Permit2NonceMismatch,
        Self::Permit2SignatureExpired,
        Self::Permit2AllowanceExpired,
        Self::Permit2CallReverted,
        Self::Permit2CallSucceeded,
        Self::Erc6492WrapperDetected,
        Self::Erc6492SignatureValid,
        Self::Erc6492SignatureInvalid,
        Self::Erc6492ValidationFailed,
        Self::ProtocolStateUnavailable,
    ];

    /// Returns the stable serialized spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DomainSeparatorMismatch => "DOMAIN_SEPARATOR_MISMATCH",
            Self::DomainChainIdMismatch => "DOMAIN_CHAIN_ID_MISMATCH",
            Self::DomainNameMismatch => "DOMAIN_NAME_MISMATCH",
            Self::DomainVersionMismatch => "DOMAIN_VERSION_MISMATCH",
            Self::DomainVersionUnexpected => "DOMAIN_VERSION_UNEXPECTED",
            Self::DomainVerifyingContractMismatch => "DOMAIN_VERIFYING_CONTRACT_MISMATCH",
            Self::DomainSaltMissing => "DOMAIN_SALT_MISSING",
            Self::DomainSaltMismatch => "DOMAIN_SALT_MISMATCH",
            Self::TypehashMismatch => "TYPEHASH_MISMATCH",
            Self::FieldOrderMismatch => "FIELD_ORDER_MISMATCH",
            Self::EncodePackedUsed => "ENCODE_PACKED_USED",
            Self::IntegerWidthMismatch => "INTEGER_WIDTH_MISMATCH",
            Self::MessageNumericPrecisionLoss => "MESSAGE_NUMERIC_PRECISION_LOSS",
            Self::SigVNotNormalized => "SIG_V_NOT_NORMALIZED",
            Self::SigVInvalid => "SIG_V_INVALID",
            Self::SigHighS => "SIG_HIGH_S",
            Self::SigZeroR => "SIG_ZERO_R",
            Self::SigZeroS => "SIG_ZERO_S",
            Self::SignerMismatch => "SIGNER_MISMATCH",
            Self::SignerIsContract => "SIGNER_IS_CONTRACT",
            Self::Erc1271SignatureValid => "ERC1271_SIGNATURE_VALID",
            Self::Erc1271SignatureInvalid => "ERC1271_SIGNATURE_INVALID",
            Self::Erc1271CallFailed => "ERC1271_CALL_FAILED",
            Self::OnchainDomainSelfInconsistent => "ONCHAIN_DOMAIN_SELF_INCONSISTENT",
            Self::OnchainDomainSource => "ONCHAIN_DOMAIN_SOURCE",
            Self::BareStructHashSigned => "BARE_STRUCT_HASH_SIGNED",
            Self::Eip712PrefixMissing => "EIP712_PREFIX_MISSING",
            Self::PersonalSignDetected => "PERSONAL_SIGN_DETECTED",
            Self::UnhashedDynamicValue => "UNHASHED_DYNAMIC_VALUE",
            Self::PermitNonceMismatch => "PERMIT_NONCE_MISMATCH",
            Self::PermitExpired => "PERMIT_EXPIRED",
            Self::DeadlineLooksLikeMilliseconds => "DEADLINE_LOOKS_LIKE_MILLISECONDS",
            Self::PermitOwnerZero => "PERMIT_OWNER_ZERO",
            Self::PermitCallReverted => "PERMIT_CALL_REVERTED",
            Self::PermitCallSucceeded => "PERMIT_CALL_SUCCEEDED",
            Self::Permit2NonceUsed => "PERMIT2_NONCE_USED",
            Self::Permit2NonceMismatch => "PERMIT2_NONCE_MISMATCH",
            Self::Permit2SignatureExpired => "PERMIT2_SIGNATURE_EXPIRED",
            Self::Permit2AllowanceExpired => "PERMIT2_ALLOWANCE_EXPIRED",
            Self::Permit2CallReverted => "PERMIT2_CALL_REVERTED",
            Self::Permit2CallSucceeded => "PERMIT2_CALL_SUCCEEDED",
            Self::Erc6492WrapperDetected => "ERC6492_WRAPPER_DETECTED",
            Self::Erc6492SignatureValid => "ERC6492_SIGNATURE_VALID",
            Self::Erc6492SignatureInvalid => "ERC6492_SIGNATURE_INVALID",
            Self::Erc6492ValidationFailed => "ERC6492_VALIDATION_FAILED",
            Self::ProtocolStateUnavailable => "PROTOCOL_STATE_UNAVAILABLE",
        }
    }
}

impl fmt::Display for FindingCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One actionable fact discovered during diagnosis.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Stable code for programmatic consumers.
    pub code: FindingCode,
    /// Importance of the finding.
    pub severity: Severity,
    /// Strength of the supporting evidence.
    pub confidence: Confidence,
    /// Concise explanation of the observed condition.
    pub message: String,
    /// Value that would verify, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// Value supplied or observed, when useful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    /// Concrete corrective action, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
    /// Deterministic proof supporting the finding, when useful to display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// Complete deterministic result returned by the diagnosis core.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// Version of this serialized contract.
    schema_version: u32,
    /// Overall outcome.
    pub verdict: Verdict,
    /// EIP-712 intermediates computed from the supplied document.
    pub computed: ComputedValues,
    /// Expected signer supplied by the caller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_signer: Option<Address>,
    /// Signer recovered from the supplied digest and signature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovered_signer: Option<Address>,
    /// Ordered diagnostic findings.
    pub findings: Vec<Finding>,
    /// Normalized input and chain/protocol evidence used for this report.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<DiagnosticContext>,
}

impl Report {
    /// Creates an empty report using the current schema version.
    #[must_use]
    pub const fn new(verdict: Verdict, computed: ComputedValues) -> Self {
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            verdict,
            computed,
            expected_signer: None,
            recovered_signer: None,
            findings: Vec::new(),
            context: None,
        }
    }

    /// Returns the serialized report schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }
}
