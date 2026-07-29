//! GitHub-flavored Markdown rendering for diagnostic reports.

use core::fmt::Write;

use crate::{Finding, Report, Verdict};

/// Renders a report for pasting into an issue, pull request, or chat.
#[must_use]
pub fn render_markdown(report: &Report) -> String {
    let mut output = String::new();
    writeln!(output, "# why712 diagnostic report").ok();
    writeln!(output).ok();
    writeln!(
        output,
        "**Verdict:** {} `{}`",
        verdict_marker(report.verdict),
        verdict_name(report.verdict)
    )
    .ok();
    writeln!(output).ok();

    if let Some(context) = &report.context {
        writeln!(output, "## Input and chain context").ok();
        writeln!(output).ok();
        writeln!(output, "| Value | Observation |").ok();
        writeln!(output, "|---|---|").ok();
        writeln!(output, "| Input | `{:?}` |", context.input_source).ok();
        if let Some(standard) = context.detected_standard {
            writeln!(output, "| Standard | `{standard:?}` |").ok();
        }
        if let Some(chain_id) = context.chain_id {
            writeln!(output, "| Chain ID | `{chain_id}` |").ok();
        }
        if let Some(block_number) = context.block_number {
            writeln!(output, "| Block | `{block_number}` |").ok();
        }
        if let Some(timestamp) = context.block_timestamp {
            writeln!(output, "| Block timestamp | `{timestamp}` |").ok();
        }
        writeln!(output).ok();
    }

    if report.expected_signer.is_some() || report.recovered_signer.is_some() {
        writeln!(output, "## Signer").ok();
        writeln!(output).ok();
        writeln!(output, "| Value | Address |").ok();
        writeln!(output, "|---|---|").ok();
        if let Some(expected) = report.expected_signer {
            writeln!(output, "| Expected | `{expected}` |").ok();
        }
        if let Some(recovered) = report.recovered_signer {
            writeln!(output, "| Recovered | `{recovered}` |").ok();
        }
        writeln!(output).ok();
    }

    writeln!(output, "## Findings").ok();
    writeln!(output).ok();
    if report.findings.is_empty() {
        writeln!(output, "No diagnostic findings.").ok();
        writeln!(output).ok();
    } else {
        for finding in &report.findings {
            render_finding(&mut output, finding);
        }
    }

    writeln!(output, "## EIP-712 computation").ok();
    writeln!(output).ok();
    writeln!(output, "| Intermediate | Value |").ok();
    writeln!(output, "|---|---|").ok();
    writeln!(
        output,
        "| `encodeType` | `{}` |",
        escape_table(&report.computed.encode_type)
    )
    .ok();
    writeln!(
        output,
        "| `typeHash` | `{}` |",
        report.computed.type_hash.as_b256()
    )
    .ok();
    writeln!(
        output,
        "| `hashStruct` | `{}` |",
        report.computed.struct_hash.as_b256()
    )
    .ok();
    writeln!(
        output,
        "| `domainSeparator` | `{}` |",
        report.computed.domain_separator.as_b256()
    )
    .ok();
    writeln!(
        output,
        "| `digest` | `{}` |",
        report.computed.digest.as_b256()
    )
    .ok();
    writeln!(output).ok();
    writeln!(
        output,
        "_Report schema v{} - generated locally by why712_",
        report.schema_version()
    )
    .ok();
    output
}

fn render_finding(output: &mut String, finding: &Finding) {
    writeln!(
        output,
        "### `{}` | {} | {} confidence",
        finding.code,
        format!("{:?}", finding.severity).to_lowercase(),
        format!("{:?}", finding.confidence).to_lowercase()
    )
    .ok();
    writeln!(output).ok();
    writeln!(output, "{}", finding.message).ok();
    writeln!(output).ok();
    if finding.expected.is_some() || finding.actual.is_some() {
        writeln!(output, "| Expected | Actual |").ok();
        writeln!(output, "|---|---|").ok();
        writeln!(
            output,
            "| {} | {} |",
            finding
                .expected
                .as_deref()
                .map_or_else(|| "-".to_owned(), escape_table),
            finding
                .actual
                .as_deref()
                .map_or_else(|| "-".to_owned(), escape_table)
        )
        .ok();
        writeln!(output).ok();
    }
    if let Some(evidence) = &finding.evidence {
        writeln!(output, "**Evidence:** {}", escape_inline(evidence)).ok();
        writeln!(output).ok();
    }
    if let Some(remediation) = &finding.remediation {
        writeln!(output, "**Fix:** {}", escape_inline(remediation)).ok();
        writeln!(output).ok();
    }
}

const fn verdict_marker(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Match => "PASS",
        Verdict::Mismatch => "FAIL",
        Verdict::Inconclusive => "UNKNOWN",
    }
}

const fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Match => "MATCH",
        Verdict::Mismatch => "MISMATCH",
        Verdict::Inconclusive => "INCONCLUSIVE",
    }
}

fn escape_table(value: &str) -> String {
    escape_inline(value).replace(['\r', '\n'], "<br>")
}

fn escape_inline(value: &str) -> String {
    value.replace('\\', "\\\\").replace('|', "\\|")
}
