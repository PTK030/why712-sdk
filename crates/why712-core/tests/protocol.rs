//! Protocol-state and wallet-request integration tests.

#![allow(clippy::panic, clippy::unwrap_used)]

use alloy_primitives::{Address, Bytes, U256, address};
use alloy_sol_types::SolValue;
use why712_core::{
    ChainClient, DiagnoseRequest, DomainProbe, ERC6492_DETECTION_SUFFIX, Eip2612Snapshot, Error,
    ExecutionCall, FindingCode, InputSource, OnchainSnapshot, Permit2AllowanceItemSnapshot,
    Permit2AllowanceRequest, Permit2AllowanceSnapshot, Permit2SignatureTransferSnapshot,
    ProtocolRequest, ProtocolSnapshot, Result, SimulationResult, TypedDataDocument, Verdict,
    acquire_protocol_snapshot, decode_erc6492_signature, diagnose_offline,
    normalize_diagnostic_input, normalize_typed_data,
};

const PERMIT: &str = include_str!("../fixtures/protocol/eip2612-stale-nonce.json");
const PROTOCOL_FIXTURES: [&str; 7] = [
    include_str!("../fixtures/protocol/eip2612-valid.json"),
    include_str!("../fixtures/protocol/eip2612-expired.json"),
    include_str!("../fixtures/protocol/eip2612-milliseconds.json"),
    include_str!("../fixtures/protocol/permit2-bitmap-used.json"),
    include_str!("../fixtures/protocol/erc1271-outcomes.json"),
    include_str!("../fixtures/protocol/erc6492-predeploy.json"),
    include_str!("../fixtures/protocol/uniswap-permit2-invalid-signer.json"),
];
const VALID_MAIL: &str = include_str!("../fixtures/examples/valid-mail.json");
const PERMIT2_INVALID_SIGNER: &str =
    include_str!("../fixtures/protocol/uniswap-permit2-invalid-signer.json");
const VALID_MAIL_SIGNATURE: &str = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";

#[test]
fn protocol_fixtures_have_primary_sources_and_valid_expected_codes() {
    for encoded in PROTOCOL_FIXTURES {
        let fixture: serde_json::Value = serde_json::from_str(encoded).unwrap();
        assert!(
            fixture["source"]
                .as_str()
                .is_some_and(|source| source.starts_with("https://"))
        );
        if let Some(typed_data) = fixture.get("typedData") {
            TypedDataDocument::parse_json(&typed_data.to_string())
                .unwrap()
                .compute()
                .unwrap();
        }
        if let Some(codes) = fixture
            .get("expectedFindings")
            .and_then(|value| value.as_array())
        {
            for code in codes {
                let code = code.as_str().unwrap();
                assert!(
                    FindingCode::ALL
                        .iter()
                        .any(|candidate| candidate.as_str() == code)
                );
            }
        }
    }
}

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
    let diagnostic = normalize_diagnostic_input(&envelope.to_string()).unwrap();
    assert_eq!(diagnostic.original_json, envelope.to_string());
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
        "spender": "0x00000000000000000000000000000000000000dd",
        "nonce": "257",
        "deadline": "2000000000"
    });
    root["types"]["PermitTransferFrom"] = serde_json::json!([
        {"name":"permitted","type":"TokenPermissions"},
        {"name":"spender","type":"address"},
        {"name":"nonce","type":"uint256"},
        {"name":"deadline","type":"uint256"}
    ]);
    root["types"]["TokenPermissions"] = serde_json::json!([
        {"name":"token","type":"address"},
        {"name":"amount","type":"uint256"}
    ]);
    let computed = TypedDataDocument::parse_json(&root.to_string())
        .unwrap()
        .compute()
        .unwrap();
    assert_eq!(
        computed.encode_type,
        "PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)"
    );
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
fn decodes_erc6492_factory_and_inner_signature() {
    let target = address!("00000000000000000000000000000000000000dd");
    let calldata = Bytes::from_static(&[0x12, 0x34]);
    let inner = Bytes::from_static(&[0xab, 0xcd, 0xef]);
    let mut encoded = (target, calldata.clone(), inner.clone()).abi_encode();
    encoded.extend_from_slice(&ERC6492_DETECTION_SUFFIX);
    let signature = format!("0x{}", hex::encode(encoded));

    let wrapper = decode_erc6492_signature(&signature).unwrap();

    assert_eq!(wrapper.target, target);
    assert_eq!(wrapper.calldata, calldata);
    assert_eq!(wrapper.inner_signature, inner);
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

#[test]
fn reports_valid_expired_and_zero_owner_eip2612_states() {
    let valid = diagnose_offline(&DiagnoseRequest {
        typed_data_json: PERMIT.to_owned(),
        onchain_snapshot: Some(snapshot(ProtocolSnapshot::Eip2612(Eip2612Snapshot {
            owner: address!("00000000000000000000000000000000000000bb"),
            signed_nonce: U256::from(7),
            onchain_nonce: Some(U256::from(7)),
            state_failure: None,
            deadline: U256::from(2_000_000_000_u64),
            block_timestamp: Some(1_700_000_100),
            simulation: Some(SimulationResult::Succeeded),
        }))),
        ..DiagnoseRequest::default()
    })
    .unwrap();
    assert_has(&valid, FindingCode::PermitCallSucceeded);
    assert!(!has(&valid, FindingCode::PermitExpired));

    let invalid = diagnose_offline(&DiagnoseRequest {
        typed_data_json: PERMIT.to_owned(),
        onchain_snapshot: Some(snapshot(ProtocolSnapshot::Eip2612(Eip2612Snapshot {
            owner: address!("0000000000000000000000000000000000000000"),
            signed_nonce: U256::from(7),
            onchain_nonce: Some(U256::from(7)),
            state_failure: None,
            deadline: U256::from(1_600_000_000_u64),
            block_timestamp: Some(1_700_000_100),
            simulation: Some(SimulationResult::Reverted {
                reason: "ERC2612ExpiredSignature".to_owned(),
            }),
        }))),
        ..DiagnoseRequest::default()
    })
    .unwrap();
    assert_eq!(invalid.verdict, Verdict::Mismatch);
    assert_has(&invalid, FindingCode::PermitExpired);
    assert_has(&invalid, FindingCode::PermitOwnerZero);
    assert_has(&invalid, FindingCode::PermitCallReverted);
}

#[test]
fn reports_all_permit2_allowance_state_and_simulation_findings() {
    let report = diagnose_offline(&DiagnoseRequest {
        typed_data_json: PERMIT.to_owned(),
        onchain_snapshot: Some(snapshot(ProtocolSnapshot::Permit2Allowance(
            Permit2AllowanceSnapshot {
                allowances: vec![Permit2AllowanceItemSnapshot {
                    signed_nonce: U256::from(8),
                    onchain_nonce: Some(U256::from(7)),
                    state_failure: None,
                    expiration: U256::from(1_600_000_000_u64),
                }],
                signature_deadline: U256::from(1_600_000_001_u64),
                block_timestamp: Some(1_700_000_100),
                simulation: Some(SimulationResult::Reverted {
                    reason: "InvalidNonce".to_owned(),
                }),
            },
        ))),
        ..DiagnoseRequest::default()
    })
    .unwrap();

    assert_eq!(report.verdict, Verdict::Mismatch);
    assert_has(&report, FindingCode::Permit2NonceMismatch);
    assert_has(&report, FindingCode::Permit2AllowanceExpired);
    assert_has(&report, FindingCode::Permit2SignatureExpired);
    assert_has(&report, FindingCode::Permit2CallReverted);
}

#[test]
fn uniswap_invalid_signer_case_reports_the_actionable_nonce_mismatch() {
    let fixture: serde_json::Value = serde_json::from_str(PERMIT2_INVALID_SIGNER).unwrap();
    let report = diagnose_offline(&DiagnoseRequest {
        typed_data_json: fixture["typedData"].to_string(),
        onchain_snapshot: Some(snapshot(ProtocolSnapshot::Permit2Allowance(
            Permit2AllowanceSnapshot {
                allowances: vec![Permit2AllowanceItemSnapshot {
                    signed_nonce: U256::from(8),
                    onchain_nonce: Some(U256::from(7)),
                    state_failure: None,
                    expiration: U256::from(2_000_000_000_u64),
                }],
                signature_deadline: U256::from(2_000_000_100_u64),
                block_timestamp: Some(1_700_000_100),
                simulation: None,
            },
        ))),
        ..DiagnoseRequest::default()
    })
    .unwrap();

    assert_has(&report, FindingCode::Permit2NonceMismatch);
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.code == FindingCode::Permit2NonceMismatch)
        .unwrap();
    assert_eq!(finding.expected.as_deref(), Some("7"));
    assert_eq!(finding.actual.as_deref(), Some("8"));
    assert!(
        finding
            .remediation
            .as_deref()
            .is_some_and(|fix| { fix.contains("allowance(owner,token,spender)") })
    );
}

struct Permit2Client;

impl ChainClient for Permit2Client {
    async fn eth_call(&self, _to: Address, _data: Bytes) -> Result<Bytes> {
        Err(Error::Rpc {
            method: "eth_call",
            reason: "block-aware call expected".to_owned(),
        })
    }

    async fn eth_call_at(&self, _to: Address, _data: Bytes, block: Option<u64>) -> Result<Bytes> {
        assert_eq!(block, Some(99));
        let mut words = Vec::new();
        words.extend_from_slice(&U256::from(10).to_be_bytes::<32>());
        words.extend_from_slice(&U256::from(2_000_000_000_u64).to_be_bytes::<32>());
        words.extend_from_slice(&U256::from(7).to_be_bytes::<32>());
        Ok(Bytes::from(words))
    }

    async fn eth_call_from_at(
        &self,
        _to: Address,
        from: Address,
        data: Bytes,
        block: Option<u64>,
    ) -> Result<Bytes> {
        assert_eq!(from, address!("00000000000000000000000000000000000000dd"));
        assert_eq!(data, Bytes::from_static(&[0xde, 0xad, 0xbe, 0xef]));
        assert_eq!(block, Some(99));
        Ok(Bytes::new())
    }

    async fn eth_chain_id(&self) -> Result<U256> {
        Ok(U256::from(1))
    }

    async fn eth_get_code(&self, _address: Address) -> Result<Bytes> {
        Ok(Bytes::new())
    }
}

#[tokio::test]
async fn permit2_simulates_exact_transaction_calldata_with_original_sender() {
    let snapshot = acquire_protocol_snapshot(
        &Permit2Client,
        address!("00000000000000000000000000000000000000aa"),
        ProtocolRequest::Permit2Allowance {
            owner: address!("00000000000000000000000000000000000000bb"),
            spender: address!("00000000000000000000000000000000000000cc"),
            details: vec![Permit2AllowanceRequest {
                token: address!("00000000000000000000000000000000000000ee"),
                nonce: U256::from(7),
                expiration: U256::from(2_000_000_000_u64),
            }],
            signature_deadline: U256::from(2_000_000_100_u64),
        },
        Some("0x00"),
        Some(&ExecutionCall {
            from: address!("00000000000000000000000000000000000000dd"),
            calldata: Bytes::from_static(&[0xde, 0xad, 0xbe, 0xef]),
        }),
        Some(99),
        Some(1_700_000_000),
    )
    .await;

    let ProtocolSnapshot::Permit2Allowance(state) = snapshot else {
        panic!("expected Permit2 allowance snapshot");
    };
    assert_eq!(state.simulation, Some(SimulationResult::Succeeded));
    assert_eq!(state.allowances[0].onchain_nonce, Some(U256::from(7)));
}

struct ArchiveRefusalClient;

impl ChainClient for ArchiveRefusalClient {
    async fn eth_call(&self, _to: Address, _data: Bytes) -> Result<Bytes> {
        Err(Error::Rpc {
            method: "eth_call",
            reason: "missing trie node".to_owned(),
        })
    }

    async fn eth_call_at(&self, _to: Address, _data: Bytes, _block: Option<u64>) -> Result<Bytes> {
        Err(Error::Rpc {
            method: "eth_call",
            reason: "missing trie node".to_owned(),
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
async fn archive_refusal_stays_inconclusive_with_rpc_evidence() {
    let protocol = acquire_protocol_snapshot(
        &ArchiveRefusalClient,
        address!("00000000000000000000000000000000000000aa"),
        ProtocolRequest::Eip2612 {
            owner: address!("00000000000000000000000000000000000000bb"),
            spender: address!("00000000000000000000000000000000000000cc"),
            value: U256::from(1),
            nonce: U256::from(7),
            deadline: U256::from(2_000_000_000_u64),
        },
        Some(VALID_MAIL_SIGNATURE),
        Some(&ExecutionCall {
            from: address!("00000000000000000000000000000000000000dd"),
            calldata: Bytes::from_static(&[0xde, 0xad, 0xbe, 0xef]),
        }),
        Some(99),
        Some(1_700_000_000),
    )
    .await;
    let report = diagnose_offline(&DiagnoseRequest {
        typed_data_json: VALID_MAIL.to_owned(),
        signature: Some(VALID_MAIL_SIGNATURE.to_owned()),
        expected_signer: Some(address!("cd2a3d9f938e13cd947ec05abc7fe734df8dd826")),
        onchain_snapshot: Some(snapshot(protocol)),
        ..DiagnoseRequest::default()
    })
    .unwrap();

    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert_has(&report, FindingCode::ProtocolStateUnavailable);
    assert!(
        report
            .findings
            .iter()
            .filter_map(|finding| finding.evidence.as_deref())
            .any(|evidence| evidence.contains("missing trie node"))
    );
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

fn has(report: &why712_core::Report, code: FindingCode) -> bool {
    report.findings.iter().any(|finding| finding.code == code)
}
