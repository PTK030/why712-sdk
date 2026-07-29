//! Transport-neutral chain probing and ERC-1271 helper tests.

#![allow(clippy::unwrap_used)]

use std::{collections::VecDeque, sync::Mutex};

use alloy_primitives::{Address, Bytes, U256, address};
use why712_core::{
    ChainClient, DiagnoseRequest, Digest, DomainProbe, Erc1271Input, Erc1271Probe, Error,
    FindingCode, Result, Verdict, acquire_onchain_snapshot, diagnose_offline,
    erc1271::{ERC1271_MAGIC_VALUE, decode_is_valid_signature, encode_is_valid_signature},
};

struct FakeChainClient {
    calls: Mutex<VecDeque<Result<Bytes>>>,
    targets: Mutex<Vec<Address>>,
}

impl ChainClient for FakeChainClient {
    async fn eth_call(&self, to: Address, _data: Bytes) -> Result<Bytes> {
        self.targets.lock().unwrap().push(to);
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

    async fn eth_block_number(&self) -> Result<u64> {
        Ok(100)
    }

    async fn eth_block_timestamp(&self, _block_number: u64) -> Result<u64> {
        Ok(1_700_000_000)
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
        targets: Mutex::new(Vec::new()),
    };
    let snapshot = acquire_onchain_snapshot(
        &client,
        address!("00000000000000000000000000000000000000aa"),
        Some(address!("00000000000000000000000000000000000000bb")),
        None,
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

#[test]
fn decodes_standard_erc1271_returns_strictly() {
    let mut valid = [0_u8; 32];
    valid[..4].copy_from_slice(ERC1271_MAGIC_VALUE.as_slice());
    assert_eq!(
        decode_is_valid_signature(&valid).unwrap(),
        Erc1271Probe::Valid
    );

    let mut invalid_return = [0_u8; 32];
    invalid_return[..4].fill(0xff);
    let invalid = decode_is_valid_signature(&invalid_return).unwrap();
    assert_eq!(
        invalid,
        Erc1271Probe::Invalid {
            returned: [0xff; 4].into()
        }
    );
    assert!(decode_is_valid_signature(&[0x16, 0x26, 0xba, 0x7e]).is_err());
}

#[tokio::test]
async fn validates_an_opaque_non_ecdsa_contract_signature() {
    let contract = address!("00000000000000000000000000000000000000aa");
    let signer = address!("00000000000000000000000000000000000000bb");
    let mut valid = vec![0_u8; 32];
    valid[..4].copy_from_slice(ERC1271_MAGIC_VALUE.as_slice());
    let client = FakeChainClient {
        calls: Mutex::new(VecDeque::from([
            Err(Error::Rpc {
                method: "eth_call",
                reason: "execution reverted".to_owned(),
            }),
            Ok(Bytes::from(vec![0x11; 32])),
            Ok(Bytes::from(valid)),
        ])),
        targets: Mutex::new(Vec::new()),
    };
    let input =
        Erc1271Input::from_signature_hex(Digest::from_b256([0x22; 32].into()), "0xaabbcc").unwrap();

    let snapshot = acquire_onchain_snapshot(&client, contract, Some(signer), Some(input))
        .await
        .unwrap();

    assert_eq!(snapshot.erc1271, Some(Erc1271Probe::Valid));
    assert_eq!(
        client.targets.lock().unwrap().as_slice(),
        &[contract, contract, signer]
    );
    let report = diagnose_offline(&DiagnoseRequest {
        typed_data_json: include_str!("../fixtures/examples/valid-mail.json").to_owned(),
        signature: Some("0xaabbcc".to_owned()),
        expected_signer: Some(signer),
        onchain_snapshot: Some(snapshot),
        ..DiagnoseRequest::default()
    })
    .unwrap();
    assert_eq!(report.verdict, Verdict::Match);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == FindingCode::Erc1271SignatureValid)
    );
}
