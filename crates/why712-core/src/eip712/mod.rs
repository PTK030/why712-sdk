//! EIP-712 parsing and hashing.

pub(crate) mod domain;
mod hash;
mod hypothesis;

pub use domain::{DomainFields, OnchainDomain};
pub use hash::{ComputedValues, Digest, DomainSeparator, StructHash, TypeHash, TypedDataDocument};
pub use hypothesis::{
    Candidate, DomainField, HypothesisContext, HypothesisHit, HypothesisKind, MAX_CANDIDATES,
    generate_candidates, search,
};
