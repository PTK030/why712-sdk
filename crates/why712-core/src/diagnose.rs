//! Pure offline diagnosis assembly.

use serde_json::Value;

use crate::{
    Confidence, DiagnoseRequest, DiagnosticContext, DomainProbe, Erc1271Probe, Erc6492Probe,
    Finding, FindingCode, HypothesisHit, HypothesisKind, OnchainSnapshot, ParsedSignature,
    ProtocolSnapshot, RecoveryEncoding, Report, Result, Severity, SimulationResult,
    TypedDataDocument, Verdict, detect_standard, is_erc6492_signature, search,
};

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Diagnoses a request without network, filesystem, clock, or randomness.
///
/// # Errors
///
/// Returns an error only when the typed data or signature cannot be parsed or hashed. Invalid
/// but parseable signatures are represented by findings.
#[allow(clippy::too_many_lines)] // This is the single ordered assembly pipeline for a report.
pub fn diagnose_offline(request: &DiagnoseRequest) -> Result<Report> {
    let document = TypedDataDocument::parse_json(&request.typed_data_json)?;
    let computed = document.compute()?;
    let signer_is_contract = request
        .onchain_snapshot
        .as_ref()
        .is_some_and(OnchainSnapshot::signer_is_contract);
    let is_erc6492 = request
        .signature
        .as_deref()
        .is_some_and(is_erc6492_signature);
    let signature = if signer_is_contract || is_erc6492 {
        None
    } else {
        request
            .signature
            .as_deref()
            .map(ParsedSignature::parse_hex)
            .transpose()?
    };
    let recovered = signature
        .as_ref()
        .and_then(|signature| signature.recover_address(&computed.digest));

    let mut findings = numeric_precision_findings(document.raw());
    if let Some(signature) = &signature {
        signature_findings(signature, &mut findings);
    }
    if let Some(snapshot) = &request.onchain_snapshot {
        onchain_findings(snapshot, &computed.domain_separator, &mut findings);
        protocol_findings(snapshot, &mut findings);
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
    if is_erc6492 {
        erc6492_findings(request.onchain_snapshot.as_ref(), &mut findings);
    }

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
    let mut verdict = if is_erc6492 {
        match request
            .onchain_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.erc6492.as_ref())
        {
            Some(Erc6492Probe::Valid) => Verdict::Match,
            Some(Erc6492Probe::Invalid) => Verdict::Mismatch,
            Some(Erc6492Probe::Unavailable { .. }) | None => Verdict::Inconclusive,
        }
    } else if signer_is_contract {
        request
            .onchain_snapshot
            .as_ref()
            .map_or(Verdict::Inconclusive, contract_signer_verdict)
    } else {
        eoa_verdict(
            signature.as_ref(),
            request.expected_signer,
            canonical_match,
            has_signature_fault,
        )
    };
    if findings
        .iter()
        .any(|finding| finding.severity == Severity::Error && is_protocol_finding(finding.code))
    {
        verdict = Verdict::Mismatch;
    } else if findings
        .iter()
        .any(|finding| finding.code == FindingCode::ProtocolStateUnavailable)
        || request
            .onchain_snapshot
            .as_ref()
            .is_some_and(protocol_simulation_is_unavailable)
    {
        verdict = Verdict::Inconclusive;
    }
    let mut report = Report::new(verdict, computed);
    report.expected_signer = request.expected_signer;
    report.recovered_signer = recovered;
    report.findings = findings;
    let detected_standard = if is_erc6492 {
        Some(crate::DetectedStandard::Erc6492)
    } else if signer_is_contract {
        Some(crate::DetectedStandard::Erc1271)
    } else {
        detect_standard(document.raw())
    };
    report.context = Some(DiagnosticContext {
        input_source: request.input_source,
        detected_standard,
        chain_id: request
            .onchain_snapshot
            .as_ref()
            .map(|snapshot| snapshot.chain_id),
        block_number: request
            .onchain_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.block_number),
        block_timestamp: request
            .onchain_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.block_timestamp),
        protocol: request
            .onchain_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.protocol.clone()),
    });
    Ok(report)
}

fn protocol_simulation_is_unavailable(snapshot: &OnchainSnapshot) -> bool {
    match snapshot.protocol.as_ref() {
        Some(ProtocolSnapshot::Eip2612(state)) => {
            matches!(state.simulation, Some(SimulationResult::Unavailable { .. }))
        }
        Some(ProtocolSnapshot::Permit2Allowance(state)) => {
            matches!(state.simulation, Some(SimulationResult::Unavailable { .. }))
        }
        Some(ProtocolSnapshot::Permit2SignatureTransfer(state)) => {
            matches!(state.simulation, Some(SimulationResult::Unavailable { .. }))
        }
        None => false,
    }
}

fn erc6492_findings(snapshot: Option<&OnchainSnapshot>, findings: &mut Vec<Finding>) {
    findings.push(protocol_finding(
        FindingCode::Erc6492WrapperDetected,
        Severity::Info,
        "The signature carries the ERC-6492 counterfactual wrapper.",
        Some("ERC-6492 validation before ERC-1271 or ECDSA".to_owned()),
        Some("ERC-6492 suffix present".to_owned()),
        "Validate the wrapper with a bounded ERC-6492-compatible eth_call.",
    ));
    match snapshot.and_then(|snapshot| snapshot.erc6492.as_ref()) {
        Some(Erc6492Probe::Valid) => findings.push(protocol_finding(
            FindingCode::Erc6492SignatureValid,
            Severity::Info,
            "The ERC-6492-compatible validator accepted the signature.",
            Some("valid".to_owned()),
            Some("valid".to_owned()),
            "No signature fix is required.",
        )),
        Some(Erc6492Probe::Invalid) => findings.push(protocol_finding(
            FindingCode::Erc6492SignatureInvalid,
            Severity::Error,
            "The ERC-6492-compatible validator rejected the signature.",
            Some("valid".to_owned()),
            Some("invalid".to_owned()),
            "Check the factory data, signer address, inner signature, and wallet authorization.",
        )),
        Some(Erc6492Probe::Unavailable { reason }) => {
            let mut finding = protocol_finding(
                FindingCode::Erc6492ValidationFailed,
                Severity::Warning,
                "ERC-6492 validation could not complete.",
                None,
                None,
                "Use a compatible RPC and keep the wrapper below the documented size limit.",
            );
            finding.evidence = Some(reason.clone());
            findings.push(finding);
        }
        None => {}
    }
}

#[allow(clippy::too_many_lines)] // The match keeps each protocol's complete finding contract adjacent.
fn protocol_findings(snapshot: &OnchainSnapshot, findings: &mut Vec<Finding>) {
    if snapshot.protocol.is_some() && snapshot.block_timestamp.is_none() {
        let reason = snapshot
            .failures
            .iter()
            .find(|failure| matches!(failure.method, "eth_blockNumber" | "eth_getBlockByNumber"))
            .map_or("block timestamp was not captured", |failure| {
                failure.reason.as_str()
            });
        state_failure_finding(Some(reason), findings);
    }
    match snapshot.protocol.as_ref() {
        Some(ProtocolSnapshot::Eip2612(state)) => {
            state_failure_finding(state.state_failure.as_deref(), findings);
            if state.owner.is_zero() {
                findings.push(protocol_finding(
                    FindingCode::PermitOwnerZero,
                    Severity::Error,
                    "EIP-2612 forbids the zero address as owner.",
                    Some("non-zero owner".to_owned()),
                    Some(state.owner.to_string()),
                    "Sign a permit for the actual token owner.",
                ));
            }
            if let Some(onchain) = state.onchain_nonce
                && onchain != state.signed_nonce
            {
                findings.push(protocol_finding(
                    FindingCode::PermitNonceMismatch,
                    Severity::Error,
                    "The signed permit nonce differs from contract state.",
                    Some(onchain.to_string()),
                    Some(state.signed_nonce.to_string()),
                    "Read nonces(owner) immediately before requesting a new signature.",
                ));
            }
            deadline_findings(
                state.deadline,
                state.block_timestamp,
                FindingCode::PermitExpired,
                "The EIP-2612 permit has expired.",
                findings,
            );
            simulation_finding(
                state.simulation.as_ref(),
                FindingCode::PermitCallSucceeded,
                FindingCode::PermitCallReverted,
                findings,
            );
        }
        Some(ProtocolSnapshot::Permit2Allowance(state)) => {
            for (index, allowance) in state.allowances.iter().enumerate() {
                state_failure_finding(allowance.state_failure.as_deref(), findings);
                if let Some(onchain) = allowance.onchain_nonce
                    && onchain != allowance.signed_nonce
                {
                    let mut finding = protocol_finding(
                        FindingCode::Permit2NonceMismatch,
                        Severity::Error,
                        "The signed Permit2 allowance nonce differs from contract state.",
                        Some(onchain.to_string()),
                        Some(allowance.signed_nonce.to_string()),
                        "Read Permit2 allowance(owner,token,spender) and sign its current nonce.",
                    );
                    finding.evidence = Some(format!("Permit2 details index: {index}"));
                    findings.push(finding);
                }
                deadline_findings(
                    allowance.expiration,
                    state.block_timestamp,
                    FindingCode::Permit2AllowanceExpired,
                    "The Permit2 allowance expiration has passed.",
                    findings,
                );
            }
            deadline_findings(
                state.signature_deadline,
                state.block_timestamp,
                FindingCode::Permit2SignatureExpired,
                "The Permit2 signature deadline has passed.",
                findings,
            );
            simulation_finding(
                state.simulation.as_ref(),
                FindingCode::Permit2CallSucceeded,
                FindingCode::Permit2CallReverted,
                findings,
            );
        }
        Some(ProtocolSnapshot::Permit2SignatureTransfer(state)) => {
            state_failure_finding(state.state_failure.as_deref(), findings);
            if let Some(bitmap) = state.nonce_bitmap {
                let bit =
                    usize::try_from(state.nonce & alloy_primitives::U256::from(255)).unwrap_or(0);
                if ((bitmap >> bit) & alloy_primitives::U256::from(1))
                    != alloy_primitives::U256::ZERO
                {
                    findings.push(protocol_finding(
                        FindingCode::Permit2NonceUsed,
                        Severity::Error,
                        "The Permit2 unordered nonce has already been used.",
                        Some("unset nonce bit".to_owned()),
                        Some(format!("bit {bit} is set")),
                        "Choose an unused unordered nonce and request a new signature.",
                    ));
                }
            }
            deadline_findings(
                state.signature_deadline,
                state.block_timestamp,
                FindingCode::Permit2SignatureExpired,
                "The Permit2 signature deadline has passed.",
                findings,
            );
            simulation_finding(
                state.simulation.as_ref(),
                FindingCode::Permit2CallSucceeded,
                FindingCode::Permit2CallReverted,
                findings,
            );
        }
        None => {}
    }
}

fn state_failure_finding(reason: Option<&str>, findings: &mut Vec<Finding>) {
    if let Some(reason) = reason {
        let mut finding = protocol_finding(
            FindingCode::ProtocolStateUnavailable,
            Severity::Warning,
            "Required protocol state could not be read.",
            None,
            None,
            "Use an RPC endpoint and block that expose the required contract state.",
        );
        finding.evidence = Some(reason.to_owned());
        findings.push(finding);
    }
}

fn deadline_findings(
    deadline: alloy_primitives::U256,
    timestamp: Option<u64>,
    expired_code: FindingCode,
    expired_message: &str,
    findings: &mut Vec<Finding>,
) {
    if deadline >= alloy_primitives::U256::from(100_000_000_000_u64)
        && deadline < alloy_primitives::U256::from(100_000_000_000_000_u64)
    {
        findings.push(protocol_finding(
            FindingCode::DeadlineLooksLikeMilliseconds,
            Severity::Warning,
            "The deadline resembles a JavaScript millisecond timestamp.",
            Some("Unix timestamp in seconds".to_owned()),
            Some(deadline.to_string()),
            "Use Math.floor(Date.now() / 1000) before adding the validity window.",
        ));
    }
    if let Some(timestamp) = timestamp
        && deadline < alloy_primitives::U256::from(timestamp)
    {
        findings.push(protocol_finding(
            expired_code,
            Severity::Error,
            expired_message,
            Some(format!(">= {timestamp}")),
            Some(deadline.to_string()),
            "Request a new signature with a future deadline.",
        ));
    }
}

fn simulation_finding(
    simulation: Option<&SimulationResult>,
    success_code: FindingCode,
    revert_code: FindingCode,
    findings: &mut Vec<Finding>,
) {
    match simulation {
        Some(SimulationResult::Succeeded) => findings.push(protocol_finding(
            success_code,
            Severity::Info,
            "The read-only verification call succeeded.",
            Some("success".to_owned()),
            Some("success".to_owned()),
            "No simulation fix is required.",
        )),
        Some(SimulationResult::Reverted { reason }) => {
            let mut finding = protocol_finding(
                revert_code,
                Severity::Error,
                "The read-only verification call reverted.",
                Some("successful eth_call".to_owned()),
                Some("revert".to_owned()),
                "Resolve the state or signature findings, then simulate again.",
            );
            finding.evidence = Some(reason.clone());
            findings.push(finding);
        }
        Some(SimulationResult::Unavailable { reason }) => {
            let mut finding = protocol_finding(
                revert_code,
                Severity::Warning,
                "The read-only verification call was unavailable.",
                None,
                None,
                "Use an RPC endpoint that supports the requested historical block.",
            );
            finding.confidence = Confidence::High;
            finding.evidence = Some(reason.clone());
            findings.push(finding);
        }
        None => {}
    }
}

fn protocol_finding(
    code: FindingCode,
    severity: Severity,
    message: &str,
    expected: Option<String>,
    actual: Option<String>,
    remediation: &str,
) -> Finding {
    Finding {
        code,
        severity,
        confidence: Confidence::High,
        message: message.to_owned(),
        expected,
        actual,
        remediation: Some(remediation.to_owned()),
        evidence: None,
    }
}

fn is_protocol_finding(code: FindingCode) -> bool {
    matches!(
        code,
        FindingCode::PermitNonceMismatch
            | FindingCode::PermitExpired
            | FindingCode::PermitOwnerZero
            | FindingCode::PermitCallReverted
            | FindingCode::Permit2NonceUsed
            | FindingCode::Permit2NonceMismatch
            | FindingCode::Permit2SignatureExpired
            | FindingCode::Permit2AllowanceExpired
            | FindingCode::Permit2CallReverted
    )
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

    contract_signer_findings(snapshot, findings);

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

fn contract_signer_verdict(snapshot: &OnchainSnapshot) -> Verdict {
    match snapshot.erc1271.as_ref() {
        Some(Erc1271Probe::Valid) => Verdict::Match,
        Some(Erc1271Probe::Invalid { .. }) => Verdict::Mismatch,
        Some(Erc1271Probe::Unavailable { .. }) | None => Verdict::Inconclusive,
    }
}

fn eoa_verdict(
    signature: Option<&ParsedSignature>,
    expected_signer: Option<alloy_primitives::Address>,
    canonical_match: bool,
    has_signature_fault: bool,
) -> Verdict {
    match (signature, expected_signer, canonical_match) {
        (Some(_), Some(_), true) if !has_signature_fault => Verdict::Match,
        (Some(_), Some(_), _) => Verdict::Mismatch,
        _ => Verdict::Inconclusive,
    }
}

fn contract_signer_findings(snapshot: &OnchainSnapshot, findings: &mut Vec<Finding>) {
    if !snapshot.signer_is_contract() {
        return;
    }
    findings.push(Finding {
        code: FindingCode::SignerIsContract,
        severity: Severity::Info,
        confidence: Confidence::High,
        message: "The expected signer has deployed runtime code.".to_owned(),
        expected: Some("ERC-1271 validation".to_owned()),
        actual: Some("contract account".to_owned()),
        remediation: snapshot.erc1271.is_none().then(|| {
            "Supply a signature so isValidSignature(bytes32,bytes) can be called.".to_owned()
        }),
        evidence: snapshot
            .signer_code
            .as_ref()
            .map(|code| format!("eth_getCode returned {} byte(s).", code.len())),
    });
    let magic = format!("0x{}", hex::encode(crate::ERC1271_MAGIC_VALUE));
    match &snapshot.erc1271 {
        Some(Erc1271Probe::Valid) => findings.push(Finding {
            code: FindingCode::Erc1271SignatureValid,
            severity: Severity::Info,
            confidence: Confidence::Certain,
            message: "The signer contract accepted the digest and signature.".to_owned(),
            expected: Some(magic.clone()),
            actual: Some(magic),
            remediation: None,
            evidence: Some(
                "isValidSignature(bytes32,bytes) returned the ERC-1271 magic value.".to_owned(),
            ),
        }),
        Some(Erc1271Probe::Invalid { returned }) => findings.push(Finding {
            code: FindingCode::Erc1271SignatureInvalid,
            severity: Severity::Error,
            confidence: Confidence::Certain,
            message: "The signer contract rejected the digest and signature.".to_owned(),
            expected: Some(magic),
            actual: Some(format!("0x{}", hex::encode(returned))),
            remediation: Some(
                "Use the digest, signature scheme, and authorization state expected by the signer contract."
                    .to_owned(),
            ),
            evidence: Some(
                "isValidSignature(bytes32,bytes) returned a non-magic bytes4 value.".to_owned(),
            ),
        }),
        Some(Erc1271Probe::Unavailable { reason }) => findings.push(Finding {
            code: FindingCode::Erc1271CallFailed,
            severity: Severity::Warning,
            confidence: Confidence::High,
            message: "The signer contract did not produce a usable ERC-1271 result.".to_owned(),
            expected: Some(magic),
            actual: None,
            remediation: Some(
                "Inspect the signer implementation, RPC response, and current authorization state."
                    .to_owned(),
            ),
            evidence: Some(reason.clone()),
        }),
        None => {}
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
