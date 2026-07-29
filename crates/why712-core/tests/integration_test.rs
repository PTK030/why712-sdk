//! End-to-end offline diagnosis over the corpus shared with CLI and WASM.

use std::{fs, path::PathBuf};

use alloy_primitives::{Address, B256};
use serde::Deserialize;
use why712_core::{DiagnoseRequest, FindingCode, HypothesisContext, Verdict, diagnose_offline};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Example {
    id: String,
    typed_data: String,
    signature: Option<String>,
    signer: Option<String>,
    fault: Option<String>,
    #[serde(default)]
    hints: Hints,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Hints {
    #[serde(default)]
    domain_salts: Vec<String>,
    #[serde(default)]
    verifying_contracts: Vec<String>,
    #[serde(default)]
    chain_ids: Vec<u64>,
}

#[test]
fn diagnoses_the_entire_shared_corpus() -> Result<(), Box<dyn std::error::Error>> {
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("examples");
    let examples: Vec<Example> =
        serde_json::from_str(&fs::read_to_string(examples_dir.join("manifest.json"))?)?;

    for example in examples {
        let request = DiagnoseRequest {
            typed_data_json: fs::read_to_string(examples_dir.join(&example.typed_data))?,
            signature: example.signature,
            expected_signer: example
                .signer
                .as_deref()
                .map(str::parse::<Address>)
                .transpose()?,
            hypothesis_context: HypothesisContext {
                chain_ids: example.hints.chain_ids,
                domain_salts: example
                    .hints
                    .domain_salts
                    .iter()
                    .map(|value| value.parse::<B256>())
                    .collect::<Result<_, _>>()?,
                verifying_contracts: example
                    .hints
                    .verifying_contracts
                    .iter()
                    .map(|value| value.parse::<Address>())
                    .collect::<Result<_, _>>()?,
            },
        };
        let report = diagnose_offline(&request)?;

        match example.fault.as_deref() {
            None => {
                assert_eq!(report.verdict, Verdict::Match, "{}", example.id);
                assert!(report.findings.is_empty(), "{}", example.id);
            }
            Some("numeric-precision-loss") => {
                assert_eq!(report.verdict, Verdict::Inconclusive, "{}", example.id);
                assert_has_code(
                    &report,
                    FindingCode::MessageNumericPrecisionLoss,
                    &example.id,
                );
            }
            Some(fault) => {
                assert_eq!(report.verdict, Verdict::Mismatch, "{}", example.id);
                let code = expected_code(fault).ok_or_else(|| {
                    std::io::Error::other(format!("unmapped fixture fault: {fault}"))
                })?;
                assert_has_code(&report, code, &example.id);
            }
        }

        insta::assert_snapshot!(example.id, serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

fn assert_has_code(report: &why712_core::Report, code: FindingCode, id: &str) {
    assert!(
        report.findings.iter().any(|finding| finding.code == code),
        "{id} did not emit {code}"
    );
}

fn expected_code(fault: &str) -> Option<FindingCode> {
    match fault {
        "wrong-chain-id" => Some(FindingCode::DomainChainIdMismatch),
        "wrong-field-order" => Some(FindingCode::FieldOrderMismatch),
        "raw-v" => Some(FindingCode::SigVNotNormalized),
        "high-s" => Some(FindingCode::SigHighS),
        "encode-packed" => Some(FindingCode::EncodePackedUsed),
        "unexpected-domain-version" => Some(FindingCode::DomainVersionUnexpected),
        "missing-domain-salt" => Some(FindingCode::DomainSaltMissing),
        "integer-width-mismatch" => Some(FindingCode::IntegerWidthMismatch),
        "unhashed-dynamic-value" => Some(FindingCode::UnhashedDynamicValue),
        _ => None,
    }
}
