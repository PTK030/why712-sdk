//! Pure offline diagnosis assembly.

use serde_json::Value;

use crate::{
    Confidence, DiagnoseRequest, DomainProbe, Finding, FindingCode, HypothesisHit, HypothesisKind,
    OnchainSnapshot, ParsedSignature, RecoveryEncoding, Report, Result, Severity,
    TypedDataDocument, Verdict, search,
};

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Diagnoses a request without network, filesystem, clock, or randomness.
///
/// # Errors
///
/// Returns an error only when the typed data or signature cannot be parsed or hashed. Invalid
/// but parseable signatures are represented by findings.
pub fn diagnose_offline(request: &DiagnoseRequest) -> Result<Report> {
    let document = TypedDataDocument::parse_json(&request.typed_data_json)?;
    let computed = document.compute()?;
    let signature = request
        .signature
        .as_deref()
        .map(ParsedSignature::parse_hex)
        .transpose()?;
    let recovered = signature
        .as_ref()
        .and_then(|signature| signature.recover_address(&computed.digest));

    let mut findings = numeric_precision_findings(document.raw());
    if let Some(signature) = &signature {
        signature_findings(signature, &mut findings);
    }
    if let Some(snapshot) = &request.onchain_snapshot {
        onchain_findings(snapshot, &computed.domain_separator, &mut findings);
    } else if let Some(failure) = &request.onchain_failure {
        findings.push(Finding {
            code: FindingCode::OnchainDomainSource,
            severity: Severity::Info,
            confidence: Confidence::High,
            message: "RPC enrichment failed; diagnosis used offline evidence.".to_owned(),
            expected: None,
            actual: None,
            remediation: Some(
                "Check the endpoint and browser CORS policy, or continue explicitly offline."
                    .to_owned(),
            ),
            evidence: Some(failure.clone()),
        });
    }

    let signer_is_contract = request
        .onchain_snapshot
        .as_ref()
        .is_some_and(OnchainSnapshot::signer_is_contract);
    let canonical_match = request
        .expected_signer
        .is_some_and(|expected| recovered == Some(expected));
    if let (Some(signature), Some(expected)) = (&signature, request.expected_signer)
        && !canonical_match
        && !signer_is_contract
        && signature.recovery().parity().is_some()
        && !signature.has_zero_r()
        && !signature.has_zero_s()
    {
        for hit in search(&document, signature, expected, &request.hypothesis_context) {
            findings.push(hypothesis_finding(&hit));
        }
    }

    if let (Some(expected), Some(actual)) = (request.expected_signer, recovered)
        && expected != actual
        && !signer_is_contract
    {
        findings.push(Finding {
            code: FindingCode::SignerMismatch,
            severity: Severity::Error,
            confidence: Confidence::Certain,
            message: "The canonical digest recovers a different signer.".to_owned(),
            expected: Some(expected.to_string()),
            actual: Some(actual.to_string()),
            remediation: Some(
                "Resolve the more specific digest findings before changing the signer.".to_owned(),
            ),
            evidence: None,
        });
    }

    let has_signature_fault = findings.iter().any(|finding| {
        matches!(
            finding.code,
            FindingCode::SigVNotNormalized
                | FindingCode::SigVInvalid
                | FindingCode::SigHighS
                | FindingCode::SigZeroR
                | FindingCode::SigZeroS
        )
    });
    let verdict = if signer_is_contract {
        Verdict::Inconclusive
    } else {
        match (
            signature.is_some(),
            request.expected_signer,
            canonical_match,
        ) {
            (true, Some(_), true) if !has_signature_fault => Verdict::Match,
            (true, Some(_), _) => Verdict::Mismatch,
            _ => Verdict::Inconclusive,
        }
    };
    let mut report = Report::new(verdict, computed);
    report.expected_signer = request.expected_signer;
    report.recovered_signer = recovered;
    report.findings = findings;
    Ok(report)
}

fn onchain_findings(
    snapshot: &OnchainSnapshot,
    local_separator: &crate::DomainSeparator,
    findings: &mut Vec<Finding>,
) {
    let (source_message, source_evidence) = match &snapshot.domain {
        DomainProbe::Erc5267 {
            separator: Some(_), ..
        } => (
            "The contract domain came from ERC-5267 and was cross-checked with DOMAIN_SEPARATOR().",
            probe_evidence(snapshot),
        ),
        DomainProbe::Erc5267 {
            separator: None, ..
        } => (
            "The contract domain came from ERC-5267; DOMAIN_SEPARATOR() was unavailable.",
            probe_evidence(snapshot),
        ),
        DomainProbe::SeparatorOnly { .. } => (
            "ERC-5267 was unavailable; the contract domain came from DOMAIN_SEPARATOR().",
            probe_evidence(snapshot),
        ),
        DomainProbe::Unavailable => (
            "Neither standard domain probe was available; diagnosis used offline evidence.",
            probe_evidence(snapshot),
        ),
    };
    findings.push(Finding {
        code: FindingCode::OnchainDomainSource,
        severity: Severity::Info,
        confidence: Confidence::High,
        message: source_message.to_owned(),
        expected: None,
        actual: None,
        remediation: None,
        evidence: source_evidence,
    });

    if snapshot.signer_is_contract() {
        findings.push(Finding {
            code: FindingCode::SignerIsContract,
            severity: Severity::Info,
            confidence: Confidence::High,
            message: "The expected signer has deployed runtime code.".to_owned(),
            expected: Some("ERC-1271 validation".to_owned()),
            actual: Some("contract account".to_owned()),
            remediation: Some(
                "Call isValidSignature(bytes32,bytes) on the signer contract.".to_owned(),
            ),
            evidence: snapshot
                .signer_code
                .as_ref()
                .map(|code| format!("eth_getCode returned {} byte(s).", code.len())),
        });
    }

    let observed = match &snapshot.domain {
        DomainProbe::Erc5267 { domain, separator } => {
            if let (Some(recomputed), Some(returned)) = (domain.separator(), separator)
                && recomputed != *returned
            {
                findings.push(Finding {
                    code: FindingCode::OnchainDomainSelfInconsistent,
                    severity: Severity::Error,
                    confidence: Confidence::High,
                    message: "ERC-5267 fields do not reproduce the contract's DOMAIN_SEPARATOR()."
                        .to_owned(),
                    expected: Some(returned.as_b256().to_string()),
                    actual: Some(recomputed.as_b256().to_string()),
                    remediation: Some(
                        "Inspect the contract or proxy domain implementation.".to_owned(),
                    ),
                    evidence: Some(format!(
                        "ERC-5267 fields bitmap: 0x{:02x}",
                        domain.fields.bits()
                    )),
                });
            }
            separator.as_ref().copied().or_else(|| domain.separator())
        }
        DomainProbe::SeparatorOnly { separator } => Some(*separator),
        DomainProbe::Unavailable => None,
    };

    if let Some(observed) = observed
        && observed != *local_separator
    {
        findings.push(Finding {
            code: FindingCode::DomainSeparatorMismatch,
            severity: Severity::Error,
            confidence: Confidence::High,
            message: "The local and on-chain domain separators differ.".to_owned(),
            expected: Some(observed.as_b256().to_string()),
            actual: Some(local_separator.as_b256().to_string()),
            remediation: Some(
                "Compare the domain fields and sign with the contract's domain.".to_owned(),
            ),
            evidence: Some(format!("Contract: {}", snapshot.contract)),
        });
    }
}

fn probe_evidence(snapshot: &OnchainSnapshot) -> Option<String> {
    (!snapshot.failures.is_empty()).then(|| {
        snapshot
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.method, failure.reason))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

fn signature_findings(signature: &ParsedSignature, findings: &mut Vec<Finding>) {
    match signature.recovery() {
        RecoveryEncoding::Raw { value } => findings.push(Finding {
            code: FindingCode::SigVNotNormalized,
            severity: Severity::Warning,
            confidence: Confidence::High,
            message: "The recovery bit uses raw 0/1 encoding.".to_owned(),
            expected: Some(if value == 0 { "27" } else { "28" }.to_owned()),
            actual: Some(value.to_string()),
            remediation: Some("Add 27 to the final recovery byte.".to_owned()),
            evidence: None,
        }),
        RecoveryEncoding::Eip155 { value, chain_id } => findings.push(Finding {
            code: FindingCode::SigVInvalid,
            severity: Severity::Error,
            confidence: Confidence::High,
            message: "The signature uses transaction-style EIP-155 recovery encoding.".to_owned(),
            expected: Some("0/1 or 27/28".to_owned()),
            actual: Some(format!("{value} (chain ID {chain_id})")),
            remediation: Some(
                "Use the recovery byte emitted for the typed-data signature.".to_owned(),
            ),
            evidence: None,
        }),
        RecoveryEncoding::Invalid { value } => findings.push(Finding {
            code: FindingCode::SigVInvalid,
            severity: Severity::Error,
            confidence: Confidence::High,
            message: "The signature recovery byte is not recognized.".to_owned(),
            expected: Some("0/1 or 27/28".to_owned()),
            actual: Some(value.to_string()),
            remediation: Some("Obtain the unmodified signature again.".to_owned()),
            evidence: None,
        }),
        RecoveryEncoding::Electrum { .. } | RecoveryEncoding::Compact { .. } => {}
    }
    if signature.has_high_s() {
        findings.push(simple_finding(
            FindingCode::SigHighS,
            Severity::Warning,
            "The signature uses a malleable high-S scalar.",
            "low-S scalar",
            "high-S scalar",
            "Normalize to low-S and flip recovery parity.",
        ));
    }
    if signature.has_zero_r() {
        findings.push(simple_finding(
            FindingCode::SigZeroR,
            Severity::Error,
            "The signature's r scalar is zero.",
            "non-zero r",
            "0",
            "Obtain a new, unmodified signature.",
        ));
    }
    if signature.has_zero_s() {
        findings.push(simple_finding(
            FindingCode::SigZeroS,
            Severity::Error,
            "The signature's s scalar is zero.",
            "non-zero s",
            "0",
            "Obtain a new, unmodified signature.",
        ));
    }
}

fn hypothesis_finding(hit: &HypothesisHit) -> Finding {
    let (message, remediation) = match &hit.hypothesis {
        HypothesisKind::ChainId { value } => (
            format!("Using chain ID {value} recovers the expected signer."),
            format!("Set domain.chainId to {value} and sign again."),
        ),
        HypothesisKind::VerifyingContract { value } => (
            format!("Using verifying contract {value} recovers the expected signer."),
            format!("Set domain.verifyingContract to {value}."),
        ),
        HypothesisKind::RemoveDomainField { field } => (
            format!(
                "Removing domain field {} recovers the expected signer.",
                field.as_str()
            ),
            format!(
                "Remove {} from the domain and its EIP712Domain type.",
                field.as_str()
            ),
        ),
        HypothesisKind::AddDomainSalt { salt } => (
            format!("Adding domain salt {salt} recovers the expected signer."),
            format!("Declare and set domain salt to {salt}."),
        ),
        HypothesisKind::DomainVersion { value } => (
            format!("Using domain version {value:?} recovers the expected signer."),
            format!("Set domain.version to {value:?}."),
        ),
        HypothesisKind::FieldOrder { type_name, fields } => (
            format!(
                "Field order {}({}) recovers the expected signer.",
                type_name,
                fields.join(",")
            ),
            "Declare the struct fields in that exact order.".to_owned(),
        ),
        HypothesisKind::Permit2Widths => (
            "Permit2 uint160/uint48 field widths recover the expected signer.".to_owned(),
            "Match Permit2's amount, expiration, and nonce widths exactly.".to_owned(),
        ),
        HypothesisKind::EncodePacked => (
            "Packed ABI encoding recovers the expected signer.".to_owned(),
            "Replace packed encoding with canonical EIP-712 encoding.".to_owned(),
        ),
        HypothesisKind::BareStructHash => (
            "The bare primary struct hash recovers the expected signer.".to_owned(),
            "Sign the complete EIP-712 domain-and-message digest.".to_owned(),
        ),
        HypothesisKind::MissingEip712Prefix => (
            "A digest without the EIP-712 prefix recovers the expected signer.".to_owned(),
            "Include the 0x1901 prefix in the final digest.".to_owned(),
        ),
        HypothesisKind::PersonalSign => (
            "EIP-191 personal signing of the JSON recovers the expected signer.".to_owned(),
            "Request typed-data signing instead of personal_sign.".to_owned(),
        ),
        HypothesisKind::UnhashedDynamicValue => (
            "Direct ABI encoding of dynamic values recovers the expected signer.".to_owned(),
            "Hash string and bytes values before EIP-712 struct encoding.".to_owned(),
        ),
    };
    Finding {
        code: hit.code,
        severity: Severity::Error,
        confidence: Confidence::Certain,
        message,
        expected: None,
        actual: None,
        remediation: Some(remediation),
        evidence: Some(format!(
            "Digest {} recovers the expected signer.",
            hit.digest.as_b256()
        )),
    }
}

fn numeric_precision_findings(root: &Value) -> Vec<Finding> {
    let mut paths = Vec::new();
    if let Some(message) = root.get("message") {
        collect_unsafe_numbers(message, "message", &mut paths);
    }
    paths
        .into_iter()
        .map(|path| Finding {
            code: FindingCode::MessageNumericPrecisionLoss,
            severity: Severity::Warning,
            confidence: Confidence::Heuristic,
            message: format!("{path} exceeds JavaScript's safe integer range."),
            expected: Some("decimal string".to_owned()),
            actual: Some("JSON number".to_owned()),
            remediation: Some("Encode the integer as a decimal string and sign again.".to_owned()),
            evidence: None,
        })
        .collect()
}

fn collect_unsafe_numbers(value: &Value, path: &str, output: &mut Vec<String>) {
    match value {
        Value::Number(number)
            if number
                .as_u64()
                .is_some_and(|value| value > MAX_SAFE_INTEGER) =>
        {
            output.push(path.to_owned());
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                collect_unsafe_numbers(value, &format!("{path}[{index}]"), output);
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                collect_unsafe_numbers(value, &format!("{path}.{key}"), output);
            }
        }
        _ => {}
    }
}

fn simple_finding(
    code: FindingCode,
    severity: Severity,
    message: &str,
    expected: &str,
    actual: &str,
    remediation: &str,
) -> Finding {
    Finding {
        code,
        severity,
        confidence: Confidence::High,
        message: message.to_owned(),
        expected: Some(expected.to_owned()),
        actual: Some(actual.to_owned()),
        remediation: Some(remediation.to_owned()),
        evidence: None,
    }
}
