//! EIP-712 parsing and hashing.

mod hash;

pub use hash::{ComputedValues, Digest, DomainSeparator, StructHash, TypeHash, TypedDataDocument};
