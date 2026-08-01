//! Human-readable terminal rendering.

use std::io::{self, Write};

use anstream::{AutoStream, ColorChoice};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use why712_core::{ComputedValues, Report, Severity, Verdict};

pub(crate) fn render_report_text(report: &Report, no_color: bool) -> Result<()> {
    let choice = if no_color {
        ColorChoice::Never
    } else {
        ColorChoice::Auto
    };
    let mut output = AutoStream::new(io::stdout(), choice).lock();
    let verdict = match report.verdict {
        Verdict::Match => "MATCH".green().bold().to_string(),
        Verdict::Mismatch => "MISMATCH".red().bold().to_string(),
        Verdict::Inconclusive => "INCONCLUSIVE".yellow().bold().to_string(),
    };
    writeln!(output, "Verdict: {verdict}")?;
    if let Some(context) = &report.context {
        writeln!(output, "Input source:     {:?}", context.input_source)?;
        if let Some(standard) = context.detected_standard {
            writeln!(output, "Detected standard: {standard:?}")?;
        }
        if let Some(block) = context.block_number {
            writeln!(output, "State block:      {block}")?;
        }
        if let Some(transaction) = &context.transaction {
            writeln!(output, "Transaction:      {}", transaction.hash)?;
            writeln!(output, "Operation:        {}", transaction.operation)?;
        }
        if let Some(wrapper) = &context.erc6492_wrapper {
            writeln!(output, "ERC-6492 target:  {}", wrapper.target)?;
        }
    }
    if let Some(expected) = report.expected_signer {
        writeln!(output, "Expected signer:  {expected}")?;
    }
    if let Some(recovered) = report.recovered_signer {
        writeln!(output, "Recovered signer: {recovered}")?;
    }
    if report.findings.is_empty() {
        writeln!(output, "Findings: none")?;
    } else {
        writeln!(output, "Findings:")?;
        for finding in &report.findings {
            let code = match finding.severity {
                Severity::Error => finding.code.to_string().red().bold().to_string(),
                Severity::Warning => finding.code.to_string().yellow().bold().to_string(),
                Severity::Info => finding.code.to_string().cyan().bold().to_string(),
            };
            writeln!(output, "  {code}: {}", finding.message)?;
            if let Some(expected) = &finding.expected {
                writeln!(output, "    expected: {expected}")?;
            }
            if let Some(actual) = &finding.actual {
                writeln!(output, "    actual:   {actual}")?;
            }
            if let Some(evidence) = &finding.evidence {
                writeln!(output, "    evidence: {evidence}")?;
            }
            if let Some(remediation) = &finding.remediation {
                writeln!(output, "    fix:      {remediation}")?;
            }
        }
    }
    writeln!(output)?;
    render_computed(&mut output, &report.computed)
}

pub(crate) fn render_computed_text(computed: &ComputedValues, no_color: bool) -> Result<()> {
    let choice = if no_color {
        ColorChoice::Never
    } else {
        ColorChoice::Auto
    };
    let mut output = AutoStream::new(io::stdout(), choice).lock();
    render_computed(&mut output, computed)
}

fn render_computed(output: &mut impl Write, computed: &ComputedValues) -> Result<()> {
    writeln!(output, "EIP-712 intermediates:")?;
    writeln!(output, "  encodeType:      {}", computed.encode_type)?;
    writeln!(
        output,
        "  typeHash:        {}",
        computed.type_hash.as_b256()
    )?;
    writeln!(
        output,
        "  encodeData:      0x{}",
        alloy_primitives::hex::encode(&computed.encode_data)
    )?;
    writeln!(
        output,
        "  structHash:      {}",
        computed.struct_hash.as_b256()
    )?;
    writeln!(
        output,
        "  domainSeparator: {}",
        computed.domain_separator.as_b256()
    )?;
    writeln!(output, "  digest:          {}", computed.digest.as_b256())
        .context("failed to write output")
}
