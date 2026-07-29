//! Validated-at-use input boundary for deterministic diagnosis.

use alloy_primitives::Address;

use crate::HypothesisContext;

/// Inputs for the pure offline diagnosis path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnoseRequest {
    /// Complete EIP-712 typed-data JSON, preserved byte-for-byte for personal-sign hypotheses.
    pub typed_data_json: String,
    /// Optional canonical or ERC-2098 signature.
    pub signature: Option<String>,
    /// Expected EOA signer, when known.
    pub expected_signer: Option<Address>,
    /// Caller-provided candidate facts, including values observed from a chain.
    pub hypothesis_context: HypothesisContext,
}
