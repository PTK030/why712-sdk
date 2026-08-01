//! Validated-at-use input boundary for deterministic diagnosis.

use alloy_primitives::Address;

use crate::{HypothesisContext, InputSource, OnchainSnapshot, TransactionContext};

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
    /// Optional plain chain observations acquired before deterministic diagnosis.
    pub onchain_snapshot: Option<OnchainSnapshot>,
    /// RPC acquisition failure retained when diagnosis deliberately falls back offline.
    pub onchain_failure: Option<String>,
    /// Origin of the normalized typed data.
    pub input_source: InputSource,
    /// Transaction evidence when the document was reconstructed from direct calldata.
    pub transaction_context: Option<TransactionContext>,
}
