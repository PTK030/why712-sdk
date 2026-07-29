//! EIP-712 parsing and hashing.

mod hash;
mod hypothesis;

pub use hash::{ComputedValues, Digest, DomainSeparator, StructHash, TypeHash, TypedDataDocument};
pub use hypothesis::{
    Candidate, DomainField, HypothesisContext, HypothesisHit, HypothesisKind, MAX_CANDIDATES,
    generate_candidates, search,
};
