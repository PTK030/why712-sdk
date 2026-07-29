//! ERC-5267 domain metadata and canonical separator reconstruction.

use std::borrow::Cow;

use alloy_primitives::{Address, B256, U256};
use alloy_sol_types::{Eip712Domain, SolCall, sol};
use serde::Serialize;

use crate::{DomainSeparator, Error, Result};

sol! {
    interface IERC5267 {
        function eip712Domain() external view returns (
            bytes1 fields,
            string name,
            string version,
            uint256 chainId,
            address verifyingContract,
            bytes32 salt,
            uint256[] extensions
        );
    }

    interface IDomainSeparator {
        function DOMAIN_SEPARATOR() external view returns (bytes32);
    }
}

/// The five standard fields selected by the ERC-5267 bitmap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct DomainFields(u8);

impl DomainFields {
    /// Bit selecting `name`.
    pub const NAME: u8 = 1 << 0;
    /// Bit selecting `version`.
    pub const VERSION: u8 = 1 << 1;
    /// Bit selecting `chainId`.
    pub const CHAIN_ID: u8 = 1 << 2;
    /// Bit selecting `verifyingContract`.
    pub const VERIFYING_CONTRACT: u8 = 1 << 3;
    /// Bit selecting `salt`.
    pub const SALT: u8 = 1 << 4;
    /// Mask containing every field defined by ERC-5267.
    pub const KNOWN_MASK: u8 =
        Self::NAME | Self::VERSION | Self::CHAIN_ID | Self::VERIFYING_CONTRACT | Self::SALT;

    /// Creates a bitmap from the byte returned by `eip712Domain()`.
    #[must_use]
    pub const fn new(bits: u8) -> Self {
        Self(bits)
    }

    /// Returns the original bitmap byte.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Returns bits not defined by ERC-5267.
    #[must_use]
    pub const fn unknown_bits(self) -> u8 {
        self.0 & !Self::KNOWN_MASK
    }

    const fn contains(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}

/// Domain metadata returned by ERC-5267.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnchainDomain {
    /// Standard-field presence bitmap.
    pub fields: DomainFields,
    /// Human-readable domain name, when selected by the bitmap.
    pub name: Option<String>,
    /// Domain version, when selected by the bitmap.
    pub version: Option<String>,
    /// Chain ID, when selected by the bitmap.
    pub chain_id: Option<U256>,
    /// Verifying contract, when selected by the bitmap.
    pub verifying_contract: Option<Address>,
    /// Domain salt, when selected by the bitmap.
    pub salt: Option<B256>,
    /// EIP numbers declaring extended domain fields.
    pub extensions: Vec<U256>,
}

impl OnchainDomain {
    /// Decodes a successful `eip712Domain()` return value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Rpc`] when the bytes are not the exact ERC-5267 ABI return shape.
    pub fn decode(data: &[u8]) -> Result<Self> {
        let value =
            IERC5267::eip712DomainCall::abi_decode_returns(data).map_err(|source| Error::Rpc {
                method: "eip712Domain()",
                reason: source.to_string(),
            })?;
        let fields = DomainFields::new(value.fields[0]);
        Ok(Self {
            fields,
            name: fields.contains(DomainFields::NAME).then_some(value.name),
            version: fields
                .contains(DomainFields::VERSION)
                .then_some(value.version),
            chain_id: fields
                .contains(DomainFields::CHAIN_ID)
                .then_some(value.chainId),
            verifying_contract: fields
                .contains(DomainFields::VERIFYING_CONTRACT)
                .then_some(value.verifyingContract),
            salt: fields.contains(DomainFields::SALT).then_some(value.salt),
            extensions: value.extensions,
        })
    }

    /// Recomputes the canonical standard EIP-712 separator.
    ///
    /// Returns `None` when unknown bitmap bits or declared extensions require semantics this
    /// version does not implement.
    #[must_use]
    pub fn separator(&self) -> Option<DomainSeparator> {
        if self.fields.unknown_bits() != 0 || !self.extensions.is_empty() {
            return None;
        }
        let domain = Eip712Domain {
            name: self.name.clone().map(Cow::Owned),
            version: self.version.clone().map(Cow::Owned),
            chain_id: self.chain_id,
            verifying_contract: self.verifying_contract,
            salt: self.salt,
        };
        Some(DomainSeparator::from_b256(domain.separator()))
    }

    pub(crate) fn calldata() -> Vec<u8> {
        IERC5267::eip712DomainCall {}.abi_encode()
    }
}

pub(crate) fn domain_separator_calldata() -> Vec<u8> {
    IDomainSeparator::DOMAIN_SEPARATORCall {}.abi_encode()
}

pub(crate) fn decode_domain_separator(data: &[u8]) -> Result<DomainSeparator> {
    let value =
        IDomainSeparator::DOMAIN_SEPARATORCall::abi_decode_returns(data).map_err(|source| {
            Error::Rpc {
                method: "DOMAIN_SEPARATOR()",
                reason: source.to_string(),
            }
        })?;
    Ok(DomainSeparator::from_b256(value))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use alloy_primitives::{FixedBytes, address};

    #[test]
    fn decodes_lsb_first_bitmap_and_recomputes_separator() {
        let returned = IERC5267::eip712DomainReturn {
            fields: FixedBytes::from([0x0d]),
            name: "Example".to_owned(),
            version: "ignored".to_owned(),
            chainId: U256::from(1),
            verifyingContract: address!("0000000000000000000000000000000000000001"),
            salt: B256::ZERO,
            extensions: Vec::new(),
        };
        let encoded = IERC5267::eip712DomainCall::abi_encode_returns(&returned);

        let domain = OnchainDomain::decode(&encoded).unwrap();

        assert_eq!(domain.name.as_deref(), Some("Example"));
        assert_eq!(domain.version, None);
        assert_eq!(domain.chain_id, Some(U256::from(1)));
        assert!(domain.verifying_contract.is_some());
        assert_eq!(domain.salt, None);
        assert!(domain.separator().is_some());
    }

    #[test]
    fn refuses_to_guess_separator_for_extensions() {
        let domain = OnchainDomain {
            fields: DomainFields::new(DomainFields::NAME),
            name: Some("Extended".to_owned()),
            version: None,
            chain_id: None,
            verifying_contract: None,
            salt: None,
            extensions: vec![U256::from(999_999)],
        };

        assert_eq!(domain.separator(), None);
    }
}
