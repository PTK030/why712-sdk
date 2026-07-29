//! ECDSA signature parsing, classification, and address recovery.

use alloy_primitives::{Address, Signature, U256};
use serde::Serialize;

use crate::{Digest, Error, Result};

/// Wire representation used by the input signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignatureFormat {
    /// Canonical 65-byte `r || s || v`.
    Canonical,
    /// ERC-2098 64-byte `r || yParityAndS`.
    Erc2098,
}

/// How the recovery bit was represented by the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RecoveryEncoding {
    /// Bare `0` or `1`.
    Raw {
        /// Original value.
        value: u8,
    },
    /// Historical `27` or `28`.
    Electrum {
        /// Original value.
        value: u8,
    },
    /// Transaction-style EIP-155 value.
    Eip155 {
        /// Original value.
        value: u8,
        /// Chain ID encoded into `v`.
        chain_id: u64,
    },
    /// Recovery bit packed into an ERC-2098 signature.
    Compact {
        /// Extracted recovery bit.
        parity: bool,
    },
    /// A byte that is not a recognized Ethereum recovery encoding.
    Invalid {
        /// Original value.
        value: u8,
    },
}

impl RecoveryEncoding {
    /// Returns the recovery bit when the encoding is recognized.
    #[must_use]
    pub const fn parity(self) -> Option<bool> {
        match self {
            Self::Raw { value } => Some(value == 1),
            Self::Electrum { value } => Some(value == 28),
            Self::Eip155 { value, .. } => Some((value - 35) % 2 == 1),
            Self::Compact { parity } => Some(parity),
            Self::Invalid { .. } => None,
        }
    }
}

/// Parsed signature components, including non-canonical states that must be diagnosed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedSignature {
    format: SignatureFormat,
    recovery: RecoveryEncoding,
    r: U256,
    s: U256,
}

impl ParsedSignature {
    /// Parses a hex-encoded 65-byte or ERC-2098 64-byte signature.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Hex`] for malformed hex and [`Error::SignatureLength`] for any length
    /// other than 64 or 65 bytes. Invalid scalar values and recovery bytes remain representable
    /// so the diagnosis layer can report them as findings.
    pub fn parse_hex(input: &str) -> Result<Self> {
        let hex = input.strip_prefix("0x").unwrap_or(input);
        let bytes = hex::decode(hex).map_err(|source| Error::Hex {
            field: "signature",
            source,
        })?;

        match bytes.len() {
            64 => {
                let signature = Signature::from_erc2098(&bytes);
                Ok(Self {
                    format: SignatureFormat::Erc2098,
                    recovery: RecoveryEncoding::Compact {
                        parity: signature.v(),
                    },
                    r: signature.r(),
                    s: signature.s(),
                })
            }
            65 => {
                let r = U256::from_be_slice(&bytes[..32]);
                let s = U256::from_be_slice(&bytes[32..64]);
                Ok(Self {
                    format: SignatureFormat::Canonical,
                    recovery: classify_v(bytes[64]),
                    r,
                    s,
                })
            }
            len => Err(Error::SignatureLength { len }),
        }
    }

    /// Returns the original wire format.
    #[must_use]
    pub const fn format(&self) -> SignatureFormat {
        self.format
    }

    /// Returns the original recovery encoding.
    #[must_use]
    pub const fn recovery(&self) -> RecoveryEncoding {
        self.recovery
    }

    /// Returns the `r` scalar.
    #[must_use]
    pub const fn r(&self) -> U256 {
        self.r
    }

    /// Returns the `s` scalar.
    #[must_use]
    pub const fn s(&self) -> U256 {
        self.s
    }

    /// Returns whether `r` is zero.
    #[must_use]
    pub fn has_zero_r(&self) -> bool {
        self.r.is_zero()
    }

    /// Returns whether `s` is zero.
    #[must_use]
    pub fn has_zero_s(&self) -> bool {
        self.s.is_zero()
    }

    /// Returns whether `s` is above the EIP-2 low-S boundary.
    #[must_use]
    pub fn has_high_s(&self) -> bool {
        Signature::new(self.r, self.s, false)
            .normalize_s()
            .is_some()
    }

    /// Recovers the signer when `v`, `r`, and `s` form a recoverable signature.
    ///
    /// Invalid signature states return `None`, not an error: they are findings for the diagnosis
    /// layer to explain.
    #[must_use]
    pub fn recover_address(&self, digest: &Digest) -> Option<Address> {
        let parity = self.recovery.parity()?;
        Signature::new(self.r, self.s, parity)
            .recover_address_from_prehash(digest.as_b256())
            .ok()
    }

    /// Returns the ERC-2098 representation when the recovery encoding is valid.
    ///
    /// High-S input is normalized because ERC-2098 requires the top bit of canonical `s` to be
    /// available for parity.
    #[must_use]
    pub fn to_erc2098(self) -> Option<[u8; 64]> {
        let parity = self.recovery.parity()?;
        Some(Signature::new(self.r, self.s, parity).as_erc2098())
    }
}

fn classify_v(value: u8) -> RecoveryEncoding {
    match value {
        0 | 1 => RecoveryEncoding::Raw { value },
        27 | 28 => RecoveryEncoding::Electrum { value },
        35.. => RecoveryEncoding::Eip155 {
            value,
            chain_id: u64::from(value - 35) / 2,
        },
        _ => RecoveryEncoding::Invalid { value },
    }
}
