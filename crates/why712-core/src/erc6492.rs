//! Bounded ERC-6492 validation through a caller-selected compatible validator.

use alloy_primitives::{Address, Bytes};
use alloy_sol_types::{SolCall, sol};

use crate::{ChainClient, Digest, Erc6492Probe};

/// Maximum accepted wrapper size before an RPC call.
pub const MAX_ERC6492_SIGNATURE_BYTES: usize = 128 * 1024;
/// Gas supplied to the read-only validator call.
pub const ERC6492_VALIDATION_GAS: u64 = 5_000_000;

sol! {
    function isValidSig(address signer, bytes32 hash, bytes signature) external returns (bool);
}

/// Executes one bounded read-only call to a caller-selected ERC-6492 validator.
pub async fn probe_erc6492<C: ChainClient>(
    client: &C,
    validator: Address,
    signer: Address,
    hash: Digest,
    signature: Bytes,
) -> Erc6492Probe {
    if signature.len() > MAX_ERC6492_SIGNATURE_BYTES {
        return Erc6492Probe::Unavailable {
            reason: format!(
                "ERC-6492 wrapper is {} bytes; limit is {} bytes",
                signature.len(),
                MAX_ERC6492_SIGNATURE_BYTES
            ),
        };
    }
    let calldata = isValidSigCall {
        signer,
        hash: *hash.as_b256(),
        signature,
    }
    .abi_encode();
    match client
        .eth_call_limited(validator, Bytes::from(calldata), ERC6492_VALIDATION_GAS)
        .await
    {
        Ok(data) => match isValidSigCall::abi_decode_returns(&data) {
            Ok(valid) if valid => Erc6492Probe::Valid,
            Ok(_) => Erc6492Probe::Invalid,
            Err(error) => Erc6492Probe::Unavailable {
                reason: format!("validator returned malformed bool: {error}"),
            },
        },
        Err(error) => Erc6492Probe::Unavailable {
            reason: error.to_string(),
        },
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Poisoning a test-only mutex is itself a failed test.
mod tests {
    use std::sync::Mutex;

    use alloy_primitives::{U256, address};

    use super::*;
    use crate::{Error, Result};

    struct Client {
        gas: Mutex<Option<u64>>,
        response: Option<Bytes>,
    }

    impl ChainClient for Client {
        async fn eth_call(&self, _to: Address, _data: Bytes) -> Result<Bytes> {
            Err(Error::Rpc {
                method: "eth_call",
                reason: "limited call expected".to_owned(),
            })
        }

        async fn eth_call_limited(&self, _to: Address, _data: Bytes, gas: u64) -> Result<Bytes> {
            *self.gas.lock().expect("gas mutex") = Some(gas);
            self.response.clone().ok_or_else(|| Error::Rpc {
                method: "eth_call",
                reason: "execution reverted".to_owned(),
            })
        }

        async fn eth_chain_id(&self) -> Result<U256> {
            Ok(U256::from(1))
        }

        async fn eth_get_code(&self, _address: Address) -> Result<Bytes> {
            Ok(Bytes::new())
        }
    }

    #[tokio::test]
    async fn caps_gas_and_decodes_the_validator_bool() {
        let client = Client {
            gas: Mutex::new(None),
            response: Some(Bytes::from(U256::from(1).to_be_bytes::<32>().to_vec())),
        };
        let result = probe_erc6492(
            &client,
            address!("00000000000000000000000000000000000000aa"),
            address!("00000000000000000000000000000000000000bb"),
            Digest::from_b256([0x11; 32].into()),
            Bytes::from_static(&[0x64, 0x92]),
        )
        .await;
        assert_eq!(result, Erc6492Probe::Valid);
        assert_eq!(
            *client.gas.lock().expect("gas mutex"),
            Some(ERC6492_VALIDATION_GAS)
        );
    }

    #[tokio::test]
    async fn refuses_oversized_wrappers_before_rpc() {
        let client = Client {
            gas: Mutex::new(None),
            response: Some(Bytes::new()),
        };
        let result = probe_erc6492(
            &client,
            address!("00000000000000000000000000000000000000aa"),
            address!("00000000000000000000000000000000000000bb"),
            Digest::from_b256([0x11; 32].into()),
            Bytes::from(vec![0; MAX_ERC6492_SIGNATURE_BYTES + 1]),
        )
        .await;
        assert!(matches!(result, Erc6492Probe::Unavailable { .. }));
        assert_eq!(*client.gas.lock().expect("gas mutex"), None);
    }

    #[tokio::test]
    async fn distinguishes_false_malformed_and_rpc_failure() {
        let cases = [
            (
                Some(Bytes::from(U256::ZERO.to_be_bytes::<32>().to_vec())),
                true,
            ),
            (Some(Bytes::from_static(&[0x01])), false),
            (None, false),
        ];
        for (response, should_be_invalid) in cases {
            let client = Client {
                gas: Mutex::new(None),
                response,
            };
            let result = probe_erc6492(
                &client,
                address!("00000000000000000000000000000000000000aa"),
                address!("00000000000000000000000000000000000000bb"),
                Digest::from_b256([0x11; 32].into()),
                Bytes::from_static(&[0x64, 0x92]),
            )
            .await;
            if should_be_invalid {
                assert_eq!(result, Erc6492Probe::Invalid);
            } else {
                assert!(matches!(result, Erc6492Probe::Unavailable { .. }));
            }
        }
    }
}
