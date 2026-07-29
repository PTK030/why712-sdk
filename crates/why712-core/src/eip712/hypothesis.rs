//! Bounded mutations that test common EIP-712 signing mistakes.

use std::collections::BTreeSet;

use alloy_primitives::{Address, B256, eip191_hash_message, keccak256};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{Digest, FindingCode, ParsedSignature, TypedDataDocument};

/// Hard upper bound for work performed by one hypothesis search.
pub const MAX_CANDIDATES: usize = 512;

const COMMON_CHAIN_IDS: [u64; 8] = [0, 1, 10, 56, 137, 8_453, 42_161, 11_155_111];

/// Optional facts supplied by the caller to enrich otherwise offline mutations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HypothesisContext {
    /// Additional chain IDs, such as the connected RPC's chain.
    pub chain_ids: Vec<u64>,
    /// Domain salts observed on-chain or otherwise known.
    pub domain_salts: Vec<B256>,
    /// Candidate verifier addresses, such as token, proxy, implementation, or Permit2.
    pub verifying_contracts: Vec<Address>,
}

/// Domain fields that can be added or removed as one mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DomainField {
    /// Human-readable protocol name.
    Name,
    /// Protocol version.
    Version,
    /// Network chain ID.
    ChainId,
    /// Contract that verifies the signature.
    VerifyingContract,
    /// Optional 32-byte domain salt.
    Salt,
}

/// Structured mutation that produced a candidate digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HypothesisKind {
    /// Replace the domain chain ID.
    ChainId {
        /// Candidate chain ID.
        value: u64,
    },
    /// Replace the domain's verifying contract.
    VerifyingContract {
        /// Candidate verifier.
        value: Address,
    },
    /// Remove one field from both the domain type and value.
    RemoveDomainField {
        /// Removed field.
        field: DomainField,
    },
    /// Add a known salt to the domain.
    AddDomainSalt {
        /// Candidate salt.
        salt: B256,
    },
    /// Replace the domain version.
    DomainVersion {
        /// Candidate version.
        value: String,
    },
    /// Reorder one declared struct.
    FieldOrder {
        /// Struct whose fields changed.
        type_name: String,
        /// Candidate field-name order.
        fields: Vec<String>,
    },
    /// Apply Permit2's `uint160`/`uint48` detail widths.
    Permit2Widths,
    /// ABI-pack the primary struct members.
    EncodePacked,
    /// Sign the bare primary struct hash.
    BareStructHash,
    /// Omit the EIP-712 `0x1901` envelope prefix.
    MissingEip712Prefix,
    /// Apply EIP-191 personal-message hashing to the original JSON text.
    PersonalSign,
    /// ABI-encode dynamic values directly instead of hashing them.
    UnhashedDynamicValue,
}

impl HypothesisKind {
    /// Finding emitted when this mutation verifies.
    #[must_use]
    pub const fn finding_code(&self) -> FindingCode {
        match self {
            Self::ChainId { .. }
            | Self::RemoveDomainField {
                field: DomainField::ChainId,
            } => FindingCode::DomainChainIdMismatch,
            Self::VerifyingContract { .. }
            | Self::RemoveDomainField {
                field: DomainField::VerifyingContract,
            } => FindingCode::DomainVerifyingContractMismatch,
            Self::RemoveDomainField {
                field: DomainField::Version,
            } => FindingCode::DomainVersionUnexpected,
            Self::RemoveDomainField {
                field: DomainField::Salt,
            }
            | Self::AddDomainSalt { .. } => FindingCode::DomainSaltMissing,
            Self::RemoveDomainField {
                field: DomainField::Name,
            } => FindingCode::DomainNameMismatch,
            Self::DomainVersion { .. } => FindingCode::DomainVersionMismatch,
            Self::FieldOrder { .. } => FindingCode::FieldOrderMismatch,
            Self::Permit2Widths => FindingCode::IntegerWidthMismatch,
            Self::EncodePacked => FindingCode::EncodePackedUsed,
            Self::BareStructHash => FindingCode::BareStructHashSigned,
            Self::MissingEip712Prefix => FindingCode::Eip712PrefixMissing,
            Self::PersonalSign => FindingCode::PersonalSignDetected,
            Self::UnhashedDynamicValue => FindingCode::UnhashedDynamicValue,
        }
    }
}

impl DomainField {
    /// Returns the EIP-712 JSON field spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Version => "version",
            Self::ChainId => "chainId",
            Self::VerifyingContract => "verifyingContract",
            Self::Salt => "salt",
        }
    }
}

/// One ranked digest to test against the expected signer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Lower values are tested and reported first.
    pub rank: u16,
    /// Mutation applied to the input.
    pub hypothesis: HypothesisKind,
    /// Digest produced by the mutation.
    pub digest: Digest,
}

/// Candidate proven by recovering the expected signer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisHit {
    /// Candidate rank.
    pub rank: u16,
    /// Proven mutation.
    pub hypothesis: HypothesisKind,
    /// Stable diagnostic code.
    pub code: FindingCode,
    /// Digest that recovered the expected signer.
    pub digest: crate::Digest,
}

/// Generates deterministic canonical EIP-712 mutations under [`MAX_CANDIDATES`].
#[must_use]
pub fn generate_candidates(
    document: &TypedDataDocument,
    context: &HypothesisContext,
) -> Vec<Candidate> {
    let mut output = Vec::new();
    let mut digests = BTreeSet::new();
    let original = document.raw();

    generate_chain_ids(original, context, &mut output, &mut digests);
    generate_verifying_contracts(original, context, &mut output, &mut digests);
    generate_domain_fields(original, context, &mut output, &mut digests);
    generate_permit2_widths(original, &mut output, &mut digests);
    generate_alternate_digests(document, &mut output, &mut digests);
    generate_field_orders(original, &mut output, &mut digests);
    output.sort_by_key(|candidate| candidate.rank);
    output
}

/// Returns every ranked mutation that recovers `expected_signer`.
#[must_use]
pub fn search(
    document: &TypedDataDocument,
    signature: &ParsedSignature,
    expected_signer: Address,
    context: &HypothesisContext,
) -> Vec<HypothesisHit> {
    generate_candidates(document, context)
        .into_iter()
        .filter_map(|candidate| {
            (signature.recover_address(&candidate.digest) == Some(expected_signer)).then(|| {
                HypothesisHit {
                    rank: candidate.rank,
                    code: candidate.hypothesis.finding_code(),
                    hypothesis: candidate.hypothesis,
                    digest: candidate.digest,
                }
            })
        })
        .collect()
}

fn push_candidate(
    value: Value,
    rank: u16,
    hypothesis: HypothesisKind,
    output: &mut Vec<Candidate>,
    digests: &mut BTreeSet<B256>,
) {
    if output.len() >= MAX_CANDIDATES {
        return;
    }
    let Ok(document) = TypedDataDocument::from_value(value) else {
        return;
    };
    let Ok(computed) = document.compute() else {
        return;
    };
    if digests.insert(*computed.digest.as_b256()) {
        output.push(Candidate {
            rank,
            hypothesis,
            digest: computed.digest,
        });
    }
}

fn push_digest_candidate(
    digest: B256,
    rank: u16,
    hypothesis: HypothesisKind,
    output: &mut Vec<Candidate>,
    digests: &mut BTreeSet<B256>,
) {
    if output.len() < MAX_CANDIDATES && digests.insert(digest) {
        output.push(Candidate {
            rank,
            hypothesis,
            digest: Digest::from_b256(digest),
        });
    }
}

fn generate_alternate_digests(
    document: &TypedDataDocument,
    output: &mut Vec<Candidate>,
    digests: &mut BTreeSet<B256>,
) {
    let Ok(computed) = document.compute() else {
        return;
    };
    let domain = *computed.domain_separator.as_b256();
    let struct_hash = *computed.struct_hash.as_b256();

    if let Ok(packed_struct_hash) = document.packed_struct_hash() {
        push_digest_candidate(
            envelope_digest(domain, packed_struct_hash, true),
            350,
            HypothesisKind::EncodePacked,
            output,
            digests,
        );
    }
    if let Ok(Some(unhashed_struct_hash)) = document.unhashed_dynamic_struct_hash() {
        push_digest_candidate(
            envelope_digest(domain, unhashed_struct_hash, true),
            355,
            HypothesisKind::UnhashedDynamicValue,
            output,
            digests,
        );
    }
    push_digest_candidate(
        struct_hash,
        360,
        HypothesisKind::BareStructHash,
        output,
        digests,
    );
    push_digest_candidate(
        envelope_digest(domain, struct_hash, false),
        370,
        HypothesisKind::MissingEip712Prefix,
        output,
        digests,
    );
    push_digest_candidate(
        eip191_hash_message(document.source_json()),
        380,
        HypothesisKind::PersonalSign,
        output,
        digests,
    );
}

fn envelope_digest(domain: B256, struct_hash: B256, with_prefix: bool) -> B256 {
    let mut input = Vec::with_capacity(if with_prefix { 66 } else { 64 });
    if with_prefix {
        // EIP-712 section "Specification of the eth_signTypedData JSON RPC" magic bytes.
        input.extend([0x19, 0x01]);
    }
    input.extend(domain);
    input.extend(struct_hash);
    keccak256(input)
}

fn generate_verifying_contracts(
    original: &Value,
    context: &HypothesisContext,
    output: &mut Vec<Candidate>,
    digests: &mut BTreeSet<B256>,
) {
    let mut contracts = BTreeSet::new();
    contracts.extend(context.verifying_contracts.iter().copied());
    for value in contracts {
        let mut candidate = original.clone();
        let Some(domain) = object_mut(&mut candidate, "domain") else {
            continue;
        };
        let encoded = value.to_string();
        if domain.get("verifyingContract").and_then(Value::as_str) == Some(&encoded) {
            continue;
        }
        domain.insert("verifyingContract".to_owned(), encoded.into());
        push_candidate(
            candidate,
            110,
            HypothesisKind::VerifyingContract { value },
            output,
            digests,
        );
    }
}

fn generate_chain_ids(
    original: &Value,
    context: &HypothesisContext,
    output: &mut Vec<Candidate>,
    digests: &mut BTreeSet<B256>,
) {
    let mut chain_ids = BTreeSet::from(COMMON_CHAIN_IDS);
    chain_ids.extend(context.chain_ids.iter().copied());
    for value in chain_ids {
        let mut candidate = original.clone();
        let Some(domain) = object_mut(&mut candidate, "domain") else {
            continue;
        };
        if domain.get("chainId") == Some(&Value::from(value)) {
            continue;
        }
        domain.insert("chainId".to_owned(), Value::from(value));
        push_candidate(
            candidate,
            100,
            HypothesisKind::ChainId { value },
            output,
            digests,
        );
    }
}

fn generate_domain_fields(
    original: &Value,
    context: &HypothesisContext,
    output: &mut Vec<Candidate>,
    digests: &mut BTreeSet<B256>,
) {
    for (name, field, rank) in [
        ("version", DomainField::Version, 200),
        ("salt", DomainField::Salt, 210),
        ("name", DomainField::Name, 220),
    ] {
        if let Some(candidate) = remove_domain_field(original, name) {
            push_candidate(
                candidate,
                rank,
                HypothesisKind::RemoveDomainField { field },
                output,
                digests,
            );
        }
    }

    for value in ["1", "2"] {
        let mut candidate = original.clone();
        let Some(domain) = object_mut(&mut candidate, "domain") else {
            continue;
        };
        if domain.get("version").and_then(Value::as_str) == Some(value) {
            continue;
        }
        domain.insert("version".to_owned(), Value::from(value));
        push_candidate(
            candidate,
            230,
            HypothesisKind::DomainVersion {
                value: value.to_owned(),
            },
            output,
            digests,
        );
    }

    for salt in &context.domain_salts {
        let mut candidate = original.clone();
        if !upsert_domain_field(&mut candidate, "salt", "bytes32", salt.to_string().into()) {
            continue;
        }
        push_candidate(
            candidate,
            205,
            HypothesisKind::AddDomainSalt { salt: *salt },
            output,
            digests,
        );
    }
}

fn generate_field_orders(
    original: &Value,
    output: &mut Vec<Candidate>,
    digests: &mut BTreeSet<B256>,
) {
    let Some(types) = original.get("types").and_then(Value::as_object) else {
        return;
    };
    for (type_name, fields) in types {
        if type_name == "EIP712Domain" {
            continue;
        }
        let Some(fields) = fields.as_array() else {
            continue;
        };
        let mut order: Vec<usize> = (0..fields.len()).collect();
        while next_permutation(&mut order) && output.len() < MAX_CANDIDATES {
            let mut candidate = original.clone();
            let Some(candidate_fields) = type_fields_mut(&mut candidate, type_name) else {
                continue;
            };
            *candidate_fields = order.iter().map(|index| fields[*index].clone()).collect();
            let names = order
                .iter()
                .filter_map(|index| fields[*index].get("name").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect();
            push_candidate(
                candidate,
                300,
                HypothesisKind::FieldOrder {
                    type_name: type_name.clone(),
                    fields: names,
                },
                output,
                digests,
            );
        }
    }
}

fn generate_permit2_widths(
    original: &Value,
    output: &mut Vec<Candidate>,
    digests: &mut BTreeSet<B256>,
) {
    let mut candidate = original.clone();
    let Some(fields) = type_fields_mut(&mut candidate, "PermitDetails") else {
        return;
    };
    let mut changed = false;
    for field in fields {
        let Some(field) = field.as_object_mut() else {
            continue;
        };
        let replacement = match field.get("name").and_then(Value::as_str) {
            Some("amount") => Some("uint160"),
            Some("expiration" | "nonce") => Some("uint48"),
            _ => None,
        };
        if let Some(replacement) = replacement {
            changed |= field.insert("type".to_owned(), replacement.into())
                != Some(Value::from(replacement));
        }
    }
    if changed {
        push_candidate(
            candidate,
            250,
            HypothesisKind::Permit2Widths,
            output,
            digests,
        );
    }
}

fn remove_domain_field(original: &Value, name: &str) -> Option<Value> {
    let mut candidate = original.clone();
    object_mut(&mut candidate, "domain")?.remove(name)?;
    let fields = type_fields_mut(&mut candidate, "EIP712Domain")?;
    fields.retain(|field| field.get("name").and_then(Value::as_str) != Some(name));
    Some(candidate)
}

fn upsert_domain_field(candidate: &mut Value, name: &str, field_type: &str, value: Value) -> bool {
    let Some(domain) = object_mut(candidate, "domain") else {
        return false;
    };
    domain.insert(name.to_owned(), value);
    let Some(fields) = type_fields_mut(candidate, "EIP712Domain") else {
        return false;
    };
    if !fields
        .iter()
        .any(|field| field.get("name").and_then(Value::as_str) == Some(name))
    {
        fields.push(serde_json::json!({ "name": name, "type": field_type }));
    }
    true
}

fn object_mut<'a>(value: &'a mut Value, key: &str) -> Option<&'a mut Map<String, Value>> {
    value.get_mut(key)?.as_object_mut()
}

fn type_fields_mut<'a>(value: &'a mut Value, type_name: &str) -> Option<&'a mut Vec<Value>> {
    value.get_mut("types")?.get_mut(type_name)?.as_array_mut()
}

fn next_permutation(values: &mut [usize]) -> bool {
    let Some(pivot) = (0..values.len().saturating_sub(1))
        .rev()
        .find(|index| values[*index] < values[*index + 1])
    else {
        return false;
    };
    let Some(swap) = (pivot + 1..values.len())
        .rev()
        .find(|index| values[pivot] < values[*index])
    else {
        return false;
    };
    values.swap(pivot, swap);
    values[pivot + 1..].reverse();
    true
}
