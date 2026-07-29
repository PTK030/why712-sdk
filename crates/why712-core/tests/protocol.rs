//! Protocol-state and wallet-request integration tests.

#![allow(clippy::unwrap_used)]

use alloy_primitives::{Bytes, U256, address};
use why712_core::{
    DiagnoseRequest, DomainProbe, ERC6492_DETECTION_SUFFIX, Eip2612Snapshot, FindingCode,
    InputSource, OnchainSnapshot, Permit2SignatureTransferSnapshot, ProtocolSnapshot,
    SimulationResult, Verdict, diagnose_offline, normalize_typed_data,
};

const PERMIT: &str = include_str!("../fixtures/protocol/eip2612-stale-nonce.json");
const VALID_MAIL: &str = include_str!("../fixtures/examples/valid-mail.json");
const VALID_MAIL_SIGNATURE: &str = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";

#[test]
fn normalizes_wallet_request_without_losing_the_typed_document() {
    let envelope = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "eth_signTypedData_v4",
        "params": ["0x00000000000000000000000000000000000000bb", PERMIT]
    });
    let (normalized, source) = normalize_typed_data(&envelope.to_string()).unwrap();
    assert_eq!(source, InputSource::WalletRequest);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&normalized).unwrap(),
        serde_json::from_str::<serde_json::Value>(PERMIT).unwrap()
    );
}

#[test]
fn diagnoses_nonce_and_millisecond_deadline_from_snapshot() {
    let request = DiagnoseRequest {
        typed_data_json: PERMIT.to_owned(),
        onchain_snapshot: Some(snapshot(ProtocolSnapshot::Eip2612(Eip2612Snapshot {
            owner: address!("00000000000000000000000000000000000000bb"),
            signed_nonce: U256::from(8),
            onchain_nonce: Some(U256::from(7)),
            state_failure: None,
            deadline: U256::from(1_700_000_000_000_u64),
            block_timestamp: Some(1_700_000_100),
            simulation: Some(SimulationResult::Reverted {
                reason: "ERC2612InvalidSigner".to_owned(),
            }),
        }))),
        ..DiagnoseRequest::default()
    };
    let report = diagnose_offline(&request).unwrap();
    assert_eq!(report.verdict, Verdict::Mismatch);
    assert_has(&report, FindingCode::PermitNonceMismatch);
    assert_has(&report, FindingCode::DeadlineLooksLikeMilliseconds);
    assert_has(&report, FindingCode::PermitCallReverted);
}

#[test]
fn diagnoses_used_permit2_bitmap_nonce() {
    let mut root: serde_json::Value = serde_json::from_str(PERMIT).unwrap();
    root["primaryType"] = "PermitTransferFrom".into();
    root["message"] = serde_json::json!({
        "permitted": {
            "token": "0x00000000000000000000000000000000000000aa",
            "amount": "1"
        },
        "nonce": "257",
        "deadline": "2000000000"
    });
    root["types"]["PermitTransferFrom"] = serde_json::json!([
        {"name":"permitted","type":"TokenPermissions"},
        {"name":"nonce","type":"uint256"},
        {"name":"deadline","type":"uint256"}
    ]);
    root["types"]["TokenPermissions"] = serde_json::json!([
        {"name":"token","type":"address"},
        {"name":"amount","type":"uint256"}
    ]);
    let request = DiagnoseRequest {
        typed_data_json: root.to_string(),
        onchain_snapshot: Some(snapshot(ProtocolSnapshot::Permit2SignatureTransfer(
            Permit2SignatureTransferSnapshot {
                nonce: U256::from(257),
                nonce_bitmap: Some(U256::from(2)),
                state_failure: None,
                signature_deadline: U256::from(2_000_000_000_u64),
                block_timestamp: Some(1_700_000_100),
                simulation: None,
            },
        ))),
        ..DiagnoseRequest::default()
    };
    let report = diagnose_offline(&request).unwrap();
    assert_has(&report, FindingCode::Permit2NonceUsed);
}

#[test]
fn detects_erc6492_before_ecdsa_length_validation() {
    let signature = format!("0x00{}", hex::encode(ERC6492_DETECTION_SUFFIX));
    let report = diagnose_offline(&DiagnoseRequest {
        typed_data_json: PERMIT.to_owned(),
        signature: Some(signature),
        ..DiagnoseRequest::default()
    })
    .unwrap();
    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert_has(&report, FindingCode::Erc6492WrapperDetected);
}

#[test]
fn missing_protocol_state_is_inconclusive_not_a_match() {
    let request = DiagnoseRequest {
        typed_data_json: PERMIT.to_owned(),
        onchain_snapshot: Some(snapshot(ProtocolSnapshot::Eip2612(Eip2612Snapshot {
            owner: address!("00000000000000000000000000000000000000bb"),
            signed_nonce: U256::from(8),
            onchain_nonce: None,
            state_failure: Some("archive state unavailable".to_owned()),
            deadline: U256::from(2_000_000_000_u64),
            block_timestamp: Some(1_700_000_100),
            simulation: None,
        }))),
        ..DiagnoseRequest::default()
    };
    let report = diagnose_offline(&request).unwrap();
    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert_has(&report, FindingCode::ProtocolStateUnavailable);
}

#[test]
fn unavailable_simulation_overrides_an_otherwise_matching_signature() {
    let request = DiagnoseRequest {
        typed_data_json: VALID_MAIL.to_owned(),
        signature: Some(VALID_MAIL_SIGNATURE.to_owned()),
        expected_signer: Some(address!("cd2a3d9f938e13cd947ec05abc7fe734df8dd826")),
        onchain_snapshot: Some(snapshot(ProtocolSnapshot::Eip2612(Eip2612Snapshot {
            owner: address!("00000000000000000000000000000000000000bb"),
            signed_nonce: U256::from(7),
            onchain_nonce: Some(U256::from(7)),
            state_failure: None,
            deadline: U256::from(2_000_000_000_u64),
            block_timestamp: Some(1_700_000_100),
            simulation: Some(SimulationResult::Unavailable {
                reason: "archive state unavailable".to_owned(),
            }),
        }))),
        ..DiagnoseRequest::default()
    };

    let report = diagnose_offline(&request).unwrap();
    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert_has(&report, FindingCode::PermitCallReverted);
}

fn snapshot(protocol: ProtocolSnapshot) -> OnchainSnapshot {
    OnchainSnapshot {
        chain_id: U256::from(1),
        contract: address!("00000000000000000000000000000000000000aa"),
        contract_code: Bytes::from_static(&[0x60]),
        signer_code: None,
        erc1271: None,
        erc6492: None,
        domain: DomainProbe::Unavailable,
        failures: Vec::new(),
        block_number: Some(1),
        block_timestamp: Some(1_700_000_100),
        protocol: Some(protocol),
    }
}

fn assert_has(report: &why712_core::Report, code: FindingCode) {
    assert!(report.findings.iter().any(|finding| finding.code == code));
}
