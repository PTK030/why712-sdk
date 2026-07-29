//! Pure ERC-1271 ABI helpers.

use alloy_primitives::{B256, Bytes, FixedBytes};
use alloy_sol_types::{SolCall, sol};

sol! {
    function isValidSignature(bytes32 hash, bytes signature) external view returns (bytes4);
}

/// Success value required by ERC-1271.
pub const ERC1271_MAGIC_VALUE: FixedBytes<4> = FixedBytes::new([0x16, 0x26, 0xba, 0x7e]);

/// Encodes `isValidSignature(bytes32,bytes)` calldata.
#[must_use]
pub fn encode_is_valid_signature(hash: B256, signature: Bytes) -> Vec<u8> {
    isValidSignatureCall { hash, signature }.abi_encode()
}
