//! Contract tests for stable report serialization and diagnostic documentation.

use std::collections::BTreeSet;

use serde_json::json;
use why712_core::{
    Confidence, Finding, FindingCode, REPORT_SCHEMA_VERSION, Report, Severity, TypedDataDocument,
    Verdict,
};

const MAIL: &str = include_str!("../fixtures/vectors/eip712-mail.json");
const DIAGNOSTICS: &str = include_str!("../../../docs/diagnostics.md");

#[test]
fn every_finding_code_has_one_catalog_entry() -> Result<(), Box<dyn std::error::Error>> {
    let headings: Vec<&str> = DIAGNOSTICS
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect();
    let unique: BTreeSet<&str> = headings.iter().copied().collect();

    assert_eq!(headings.len(), unique.len(), "duplicate diagnostic heading");
    assert_eq!(unique.len(), FindingCode::ALL.len());
    for code in FindingCode::ALL {
        assert!(
            unique.contains(code.as_str()),
            "{} is missing from docs/diagnostics.md",
            code.as_str()
        );
        assert_eq!(serde_json::to_value(code)?, json!(code.as_str()));
    }
    Ok(())
}

#[test]
fn report_json_is_a_versioned_camel_case_contract() -> Result<(), Box<dyn std::error::Error>> {
    let computed = TypedDataDocument::parse_json(MAIL)?.compute()?;
    let mut report = Report::new(Verdict::Mismatch, computed);
    report.findings.push(Finding {
        code: FindingCode::DomainChainIdMismatch,
        severity: Severity::Error,
        confidence: Confidence::Certain,
        message: "The signature verifies on chain 1.".to_owned(),
        expected: Some("1".to_owned()),
        actual: Some("137".to_owned()),
        remediation: Some("Switch to chain 1 and sign again.".to_owned()),
        evidence: Some("Candidate digest recovered the expected signer.".to_owned()),
    });

    let json = serde_json::to_value(&report)?;
    assert_eq!(json["schemaVersion"], REPORT_SCHEMA_VERSION);
    assert_eq!(json["verdict"], "MISMATCH");
    assert!(json.get("expectedSigner").is_none());
    assert!(json.get("recoveredSigner").is_none());
    assert_eq!(
        json["findings"][0]["code"],
        FindingCode::DomainChainIdMismatch.as_str()
    );
    assert_eq!(json["findings"][0]["severity"], "ERROR");
    assert_eq!(json["findings"][0]["confidence"], "CERTAIN");
    assert!(
        json["computed"]["encodeData"]
            .as_str()
            .is_some_and(|value| value.starts_with("0x"))
    );
    assert_eq!(serde_json::from_value::<Report>(json)?, report);
    Ok(())
}
