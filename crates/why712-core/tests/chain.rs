//! Transport-neutral chain probing and ERC-1271 helper tests.

#![allow(clippy::unwrap_used)]

use std::{collections::VecDeque, sync::Mutex};

use alloy_primitives::{Address, Bytes, U256, address};
use why712_core::{
    ChainClient, DomainProbe, Error, Result, acquire_onchain_snapshot,
    erc1271::{ERC1271_MAGIC_VALUE, encode_is_valid_signature},
};

struct FakeChainClient {
    calls: Mutex<VecDeque<Result<Bytes>>>,
}

impl ChainClient for FakeChainClient {
    async fn eth_call(&self, _to: Address, _data: Bytes) -> Result<Bytes> {
        self.calls.lock().unwrap().pop_front().unwrap()
    }

    async fn eth_chain_id(&self) -> Result<U256> {
        Ok(U256::from(1))
    }

    async fn eth_get_code(&self, address: Address) -> Result<Bytes> {
        if address == address!("00000000000000000000000000000000000000bb") {
            Ok(Bytes::from_static(&[0x60, 0x00]))
        } else {
            Ok(Bytes::from_static(&[0x60]))
        }
    }
}

#[tokio::test]
async fn falls_back_to_domain_separator_and_detects_contract_signer() {
    let client = FakeChainClient {
        calls: Mutex::new(VecDeque::from([
            Err(Error::Rpc {
                method: "eth_call",
                reason: "execution reverted".to_owned(),
            }),
            Ok(Bytes::from(vec![0x11; 32])),
        ])),
    };
    let snapshot = acquire_onchain_snapshot(
        &client,
        address!("00000000000000000000000000000000000000aa"),
        Some(address!("00000000000000000000000000000000000000bb")),
    )
    .await
    .unwrap();

    assert_eq!(snapshot.chain_id, U256::from(1));
    assert!(snapshot.signer_is_contract());
    assert_eq!(snapshot.failures.len(), 1);
    assert!(matches!(snapshot.domain, DomainProbe::SeparatorOnly { .. }));
}

#[test]
fn encodes_the_erc1271_selector_and_magic_value() {
    let calldata = encode_is_valid_signature([0x22; 32].into(), Bytes::from_static(&[0xaa, 0xbb]));

    assert_eq!(&calldata[..4], ERC1271_MAGIC_VALUE.as_slice());
    assert_eq!(ERC1271_MAGIC_VALUE.as_slice(), &[0x16, 0x26, 0xba, 0x7e]);
}
