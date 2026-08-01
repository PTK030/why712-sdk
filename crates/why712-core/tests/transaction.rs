//! Transaction reconstruction and historical-state tests.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Mutex;

use alloy_primitives::{Address, B256, Bytes, FixedBytes, U256, Uint, address, b256, keccak256};
use alloy_sol_types::{SolCall, sol};
use why712_core::{
    ChainClient, ChainTransaction, Error, InputSource, Result, reconstruct_transaction,
};

sol! {
    interface Eip2612Direct {
        function permit(
            address owner,
            address spender,
            uint256 value,
            uint256 deadline,
            uint8 v,
            bytes32 r,
            bytes32 s
        ) external;
    }
    interface Erc5267Probe {
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
    interface Permit2Single {
        struct PermitDetails {
            address token;
            uint160 amount;
            uint48 expiration;
            uint48 nonce;
        }
        struct PermitSingle {
            PermitDetails details;
            address spender;
            uint256 sigDeadline;
        }
        function permit(address owner, PermitSingle permitSingle, bytes signature) external;
    }
}

const HASH: B256 = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
const TOKEN: Address = address!("00000000000000000000000000000000000000aa");
const OWNER: Address = address!("00000000000000000000000000000000000000bb");
const SPENDER: Address = address!("00000000000000000000000000000000000000cc");

struct Client {
    transaction: ChainTransaction,
    call_blocks: Mutex<Vec<Option<u64>>>,
}

impl ChainClient for Client {
    async fn eth_call(&self, _to: Address, _data: Bytes) -> Result<Bytes> {
        Err(Error::Rpc {
            method: "eth_call",
            reason: "historical call expected".to_owned(),
        })
    }

    async fn eth_call_at(&self, _to: Address, data: Bytes, block: Option<u64>) -> Result<Bytes> {
        self.call_blocks.lock().expect("call blocks").push(block);
        if data.starts_with(&Erc5267Probe::eip712DomainCall::SELECTOR) {
            let returned = Erc5267Probe::eip712DomainReturn {
                fields: FixedBytes::from([0x0f]),
                name: "Token".to_owned(),
                version: "1".to_owned(),
                chainId: U256::from(1),
                verifyingContract: TOKEN,
                salt: B256::ZERO,
                extensions: Vec::new(),
            };
            return Ok(Bytes::from(
                Erc5267Probe::eip712DomainCall::abi_encode_returns(&returned),
            ));
        }
        if data.starts_with(&keccak256("nonces(address)".as_bytes())[..4]) {
            return Ok(Bytes::from(U256::from(7).to_be_bytes::<32>().to_vec()));
        }
        Err(Error::Rpc {
            method: "eth_call",
            reason: "unexpected calldata".to_owned(),
        })
    }

    async fn eth_chain_id(&self) -> Result<U256> {
        Ok(U256::from(1))
    }

    async fn eth_get_code(&self, _address: Address) -> Result<Bytes> {
        Ok(Bytes::new())
    }

    async fn eth_block_number(&self) -> Result<u64> {
        Ok(123)
    }

    async fn eth_block_timestamp(&self, block_number: u64) -> Result<u64> {
        assert!(matches!(block_number, 99 | 123));
        Ok(1_700_000_000)
    }

    async fn eth_get_transaction_by_hash(&self, hash: B256) -> Result<ChainTransaction> {
        assert_eq!(hash, HASH);
        Ok(self.transaction.clone())
    }
}

#[tokio::test]
async fn mined_eip2612_uses_parent_state_and_reconstructs_the_document() {
    let calldata = Eip2612Direct::permitCall {
        owner: OWNER,
        spender: SPENDER,
        value: U256::from(1),
        deadline: U256::from(2_000_000_000_u64),
        v: 27,
        r: B256::from([0x11; 32]),
        s: B256::from([0x22; 32]),
    }
    .abi_encode();
    let client = Client {
        transaction: ChainTransaction {
            original_json: "{\"hash\":\"0xaaaa\"}".to_owned(),
            hash: HASH,
            to: TOKEN,
            from: SPENDER,
            input: Bytes::from(calldata),
            block_number: Some(100),
        },
        call_blocks: Mutex::new(Vec::new()),
    };

    let reconstructed = reconstruct_transaction(&client, HASH).await.unwrap();

    assert_eq!(reconstructed.input.source, InputSource::Transaction);
    assert_eq!(reconstructed.input.original_json, "{\"hash\":\"0xaaaa\"}");
    assert_eq!(reconstructed.context.state_block, 99);
    assert!(!reconstructed.context.pending);
    assert_eq!(reconstructed.context.operation, "EIP2612_PERMIT");
    assert_eq!(reconstructed.signer, OWNER);
    let typed: serde_json::Value =
        serde_json::from_str(&reconstructed.input.typed_data_json).unwrap();
    assert_eq!(typed["message"]["nonce"], "7");
    assert_eq!(typed["domain"]["name"], "Token");
    assert!(
        client
            .call_blocks
            .lock()
            .expect("call blocks")
            .iter()
            .all(|block| *block == Some(99))
    );
}

#[tokio::test]
async fn unknown_selector_is_rejected_without_heuristic_decoding() {
    let client = Client {
        transaction: ChainTransaction {
            original_json: "{\"hash\":\"0xaaaa\"}".to_owned(),
            hash: HASH,
            to: TOKEN,
            from: SPENDER,
            input: Bytes::from_static(&[0xde, 0xad, 0xbe, 0xef]),
            block_number: None,
        },
        call_blocks: Mutex::new(Vec::new()),
    };

    let error = reconstruct_transaction(&client, HASH).await.unwrap_err();
    assert!(error.to_string().contains("not an allowlisted direct"));
}

#[tokio::test]
async fn pending_permit2_uses_latest_state() {
    let calldata = Permit2Single::permitCall {
        owner: OWNER,
        permitSingle: Permit2Single::PermitSingle {
            details: Permit2Single::PermitDetails {
                token: TOKEN,
                amount: Uint::<160, 3>::from_limbs([1, 0, 0]),
                expiration: Uint::<48, 1>::from_limbs([2_000_000_000]),
                nonce: Uint::<48, 1>::from_limbs([3]),
            },
            spender: SPENDER,
            sigDeadline: U256::from(2_000_000_100_u64),
        },
        signature: Bytes::from_static(&[0x11; 65]),
    }
    .abi_encode();
    let client = Client {
        transaction: ChainTransaction {
            original_json: "{\"hash\":\"0xaaaa\"}".to_owned(),
            hash: HASH,
            to: address!("00000000000000000000000000000000000000dd"),
            from: SPENDER,
            input: Bytes::from(calldata),
            block_number: None,
        },
        call_blocks: Mutex::new(Vec::new()),
    };

    let reconstructed = reconstruct_transaction(&client, HASH).await.unwrap();

    assert!(reconstructed.context.pending);
    assert_eq!(reconstructed.context.state_block, 123);
    assert_eq!(reconstructed.context.operation, "PERMIT2_PERMIT_SINGLE");
    let typed: serde_json::Value =
        serde_json::from_str(&reconstructed.input.typed_data_json).unwrap();
    assert_eq!(typed["primaryType"], "PermitSingle");
    assert_eq!(typed["message"]["details"]["nonce"], "3");
}
