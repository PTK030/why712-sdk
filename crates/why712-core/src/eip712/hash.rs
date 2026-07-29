//! Thin wrapper around `alloy-dyn-abi` that preserves diagnostic intermediates.

use alloy_dyn_abi::{DynSolValue, eip712::TypedData};
use alloy_primitives::B256;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use serde_json::Value;

use crate::{Error, Result};

/// The Keccak-256 hash of an EIP-712 `encodeType` string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TypeHash(B256);

impl TypeHash {
    /// Returns the underlying 32-byte value.
    #[must_use]
    pub const fn as_b256(&self) -> &B256 {
        &self.0
    }
}

/// The EIP-712 `hashStruct` of the primary message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct StructHash(B256);

impl StructHash {
    /// Returns the underlying 32-byte value.
    #[must_use]
    pub const fn as_b256(&self) -> &B256 {
        &self.0
    }
}

/// The EIP-712 domain separator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct DomainSeparator(B256);

impl DomainSeparator {
    /// Creates a separator from an already computed 32-byte hash.
    #[must_use]
    pub const fn from_b256(value: B256) -> Self {
        Self(value)
    }

    /// Returns the underlying 32-byte value.
    #[must_use]
    pub const fn as_b256(&self) -> &B256 {
        &self.0
    }
}

/// The final EIP-712 signing digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Digest(B256);

impl Digest {
    /// Creates a digest from an already computed 32-byte hash.
    #[must_use]
    pub const fn from_b256(value: B256) -> Self {
        Self(value)
    }

    /// Returns the underlying 32-byte value.
    #[must_use]
    pub const fn as_b256(&self) -> &B256 {
        &self.0
    }
}

/// Every EIP-712 intermediate needed to explain a hashing mismatch.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedValues {
    /// Canonical type description for the primary message.
    pub encode_type: String,
    /// Hash of [`Self::encode_type`].
    pub type_hash: TypeHash,
    /// Encoded message fields, without the type hash.
    #[serde(
        serialize_with = "serialize_hex_bytes",
        deserialize_with = "deserialize_hex_bytes"
    )]
    pub encode_data: Vec<u8>,
    /// Hash of the type hash followed by [`Self::encode_data`].
    pub struct_hash: StructHash,
    /// Hash of the signing domain.
    pub domain_separator: DomainSeparator,
    /// Final hash signed by an EIP-712 signer.
    pub digest: Digest,
}

/// A validated dynamic EIP-712 document.
#[derive(Clone, Debug)]
pub struct TypedDataDocument {
    inner: TypedData,
    raw: Value,
    source_json: String,
}

impl TypedDataDocument {
    /// Parses an `eth_signTypedData` JSON document without performing any I/O.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TypedDataJson`] for invalid JSON and [`Error::TypedDataShape`] when the
    /// JSON is not a valid EIP-712 document.
    pub fn parse_json(json: &str) -> Result<Self> {
        let value = serde_json::from_str(json).map_err(|source| Error::TypedDataJson { source })?;
        Self::from_value_and_source(value, json.to_owned())
    }

    pub(crate) fn from_value(value: Value) -> Result<Self> {
        let source_json =
            serde_json::to_string(&value).map_err(|source| Error::TypedDataShape {
                reason: source.to_string(),
            })?;
        Self::from_value_and_source(value, source_json)
    }

    fn from_value_and_source(value: Value, source_json: String) -> Result<Self> {
        let inner =
            serde_json::from_value(value.clone()).map_err(|source| Error::TypedDataShape {
                reason: source.to_string(),
            })?;
        Ok(Self {
            inner,
            raw: value,
            source_json,
        })
    }

    /// Returns the validated source tree for protocol recognition and evidence extraction.
    #[must_use]
    pub const fn raw(&self) -> &Value {
        &self.raw
    }

    pub(crate) fn packed_struct_hash(&self) -> Result<B256> {
        let mut encoded = self
            .inner
            .type_hash()
            .map_err(|source| shape_error(&source))?
            .to_vec();
        encoded.extend(
            self.inner
                .coerce()
                .map_err(|source| shape_error(&source))?
                .abi_encode_packed(),
        );
        Ok(alloy_primitives::keccak256(encoded))
    }

    pub(crate) fn unhashed_dynamic_struct_hash(&self) -> Result<Option<B256>> {
        let type_hash = self
            .inner
            .type_hash()
            .map_err(|source| shape_error(&source))?;
        let value = self.inner.coerce().map_err(|source| shape_error(&source))?;
        let DynSolValue::CustomStruct { tuple, .. } = value else {
            return Ok(None);
        };
        let mut values = Vec::with_capacity(tuple.len() + 1);
        values.push(DynSolValue::FixedBytes(type_hash, 32));
        values.extend(tuple);
        Ok(Some(alloy_primitives::keccak256(
            DynSolValue::Tuple(values).abi_encode_params(),
        )))
    }

    pub(crate) fn source_json(&self) -> &str {
        &self.source_json
    }

    /// Computes the canonical EIP-712 encoding and every diagnostic intermediate.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TypedDataShape`] when a declared type cannot be resolved or a message
    /// value cannot be coerced to its declared type.
    pub fn compute(&self) -> Result<ComputedValues> {
        let encode_type = self
            .inner
            .encode_type()
            .map_err(|source| shape_error(&source))?;
        let type_hash = TypeHash(
            self.inner
                .type_hash()
                .map_err(|source| shape_error(&source))?,
        );
        let encode_data = self
            .inner
            .encode_data()
            .map_err(|source| shape_error(&source))?;
        let struct_hash = StructHash(
            self.inner
                .hash_struct()
                .map_err(|source| shape_error(&source))?,
        );
        let domain_separator = DomainSeparator(self.inner.domain().separator());
        let digest = Digest(
            self.inner
                .eip712_signing_hash()
                .map_err(|source| shape_error(&source))?,
        );

        Ok(ComputedValues {
            encode_type,
            type_hash,
            encode_data,
            struct_hash,
            domain_separator,
            digest,
        })
    }
}

fn serialize_hex_bytes<S>(value: &[u8], serializer: S) -> core::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("0x{}", hex::encode(value)))
}

fn deserialize_hex_bytes<'de, D>(deserializer: D) -> core::result::Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    hex::decode(value.strip_prefix("0x").unwrap_or(&value)).map_err(D::Error::custom)
}

fn shape_error(source: &alloy_dyn_abi::Error) -> Error {
    Error::TypedDataShape {
        reason: source.to_string(),
    }
}
