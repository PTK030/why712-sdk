//! `why712` command-line entry point.

mod output;

use std::{
    ffi::OsStr,
    fmt::Write as _,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use alloy_primitives::Address;
use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use why712_core::{
    DiagnoseRequest, DomainProbe, Erc1271Input, FindingCode, HypothesisContext, OnchainSnapshot,
    TypedDataDocument, Verdict, acquire_onchain_snapshot, diagnose_offline, render_markdown,
    rpc::AlloyChainClient,
};

use crate::output::{render_computed_text, render_report_text};

const EXIT_MISMATCH: u8 = 1;
const EXIT_USAGE_OR_IO: u8 = 2;
const EXAMPLES_MANIFEST: &str = include_str!("../../why712-core/fixtures/examples/manifest.json");
const DIAGNOSTICS: &str = include_str!("../../../docs/diagnostics.md");

#[derive(Debug, Parser)]
#[command(name = "why712", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Diagnose a signature against typed data and optionally a live contract.
    Check(CheckArgs),
    /// Print canonical EIP-712 intermediate hashes without verifying a signature.
    Digest(DigestArgs),
    /// List the bundled reproducible examples.
    Examples(ExamplesArgs),
    /// Explain a stable diagnostic code.
    Explain(ExplainArgs),
}

#[derive(Debug, Args)]
struct CheckArgs {
    /// Typed-data JSON file, or `-` for stdin.
    #[arg(long, value_name = "FILE")]
    typed_data: PathBuf,
    /// Canonical 65-byte or compact ERC-2098 signature.
    #[arg(long)]
    signature: Option<String>,
    /// Contract whose domain is used for verification.
    #[arg(long)]
    contract: Option<Address>,
    /// Expected EOA or ERC-1271 signer.
    #[arg(long)]
    signer: Option<Address>,
    /// HTTP JSON-RPC endpoint used to acquire chain evidence.
    #[arg(long, env = "WHY712_RPC_URL")]
    rpc: Option<String>,
    /// Candidate chain ID to test during offline hypothesis search.
    #[arg(long)]
    chain_id: Option<u64>,
    /// Never contact a chain, even when an RPC environment variable is set.
    #[arg(long)]
    offline: bool,
    /// Report representation.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
    /// Disable terminal colors.
    #[arg(long)]
    no_color: bool,
}

#[derive(Debug, Args)]
struct DigestArgs {
    /// Typed-data JSON file, or `-` for stdin.
    #[arg(long, value_name = "FILE")]
    typed_data: PathBuf,
    /// Intermediate representation.
    #[arg(long, value_enum, default_value_t = DigestFormat::Text)]
    format: DigestFormat,
    /// Disable terminal colors.
    #[arg(long)]
    no_color: bool,
}

#[derive(Debug, Args)]
struct ExamplesArgs {
    /// Emit the complete example manifest as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ExplainArgs {
    /// Stable code such as `DOMAIN_CHAIN_ID_MISMATCH`.
    code: String,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Markdown,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DigestFormat {
    Text,
    Json,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExampleSummary {
    id: String,
    title: String,
    fault: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(anstream::stderr().lock(), "error: {error:#}");
            ExitCode::from(EXIT_USAGE_OR_IO)
        }
    }
}

async fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Command::Check(args) => check(args).await,
        Command::Digest(args) => digest(&args),
        Command::Examples(args) => examples(&args),
        Command::Explain(args) => explain(&args),
    }
}

async fn check(args: CheckArgs) -> Result<ExitCode> {
    if !args.offline && args.rpc.is_some() && args.contract.is_none() {
        bail!("--rpc requires --contract unless --offline is set");
    }
    let typed_data_json = read_input(&args.typed_data)?;
    let mut hypothesis_context = HypothesisContext::default();
    if let Some(chain_id) = args.chain_id {
        hypothesis_context.chain_ids.push(chain_id);
    }
    if let Some(contract) = args.contract {
        hypothesis_context.verifying_contracts.push(contract);
    }

    let (onchain_snapshot, onchain_failure) = if !args.offline
        && let (Some(endpoint), Some(contract)) = (&args.rpc, args.contract)
    {
        let erc1271_input = match (args.signer, args.signature.as_deref()) {
            (Some(_), Some(signature)) => {
                let digest = TypedDataDocument::parse_json(&typed_data_json)?
                    .compute()?
                    .digest;
                Some(Erc1271Input::from_signature_hex(digest, signature)?)
            }
            _ => None,
        };
        acquire_or_warn(endpoint, contract, args.signer, erc1271_input).await
    } else {
        (None, None)
    };
    if let Some(snapshot) = &onchain_snapshot {
        enrich_hypotheses(snapshot, &mut hypothesis_context);
    }

    let request = DiagnoseRequest {
        typed_data_json,
        signature: args.signature,
        expected_signer: args.signer,
        hypothesis_context,
        onchain_snapshot,
        onchain_failure,
    };
    let report = diagnose_offline(&request)?;
    match args.format {
        OutputFormat::Text => render_report_text(&report, args.no_color)?,
        OutputFormat::Json => write_stdout(&serde_json::to_string_pretty(&report)?)?,
        OutputFormat::Markdown => write_stdout(&render_markdown(&report))?,
    }

    Ok(if report.verdict == Verdict::Match {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_MISMATCH)
    })
}

async fn acquire_or_warn(
    endpoint: &str,
    contract: Address,
    signer: Option<Address>,
    erc1271_input: Option<Erc1271Input>,
) -> (Option<OnchainSnapshot>, Option<String>) {
    let result = match AlloyChainClient::connect(endpoint).await {
        Ok(client) => acquire_onchain_snapshot(&client, contract, signer, erc1271_input).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(snapshot) => (Some(snapshot), None),
        Err(error) => {
            let failure = error.to_string();
            let _ = writeln!(
                anstream::stderr().lock(),
                "warning: RPC enrichment failed; continuing offline: {error}"
            );
            (None, Some(failure))
        }
    }
}

fn enrich_hypotheses(snapshot: &OnchainSnapshot, context: &mut HypothesisContext) {
    if let Ok(chain_id) = u64::try_from(snapshot.chain_id) {
        context.chain_ids.push(chain_id);
    }
    context.verifying_contracts.push(snapshot.contract);
    if let DomainProbe::Erc5267 { domain, .. } = &snapshot.domain {
        if let Some(chain_id) = domain.chain_id.and_then(|value| u64::try_from(value).ok()) {
            context.chain_ids.push(chain_id);
        }
        if let Some(verifying_contract) = domain.verifying_contract {
            context.verifying_contracts.push(verifying_contract);
        }
        if let Some(salt) = domain.salt {
            context.domain_salts.push(salt);
        }
    }
}

fn digest(args: &DigestArgs) -> Result<ExitCode> {
    let json = read_input(&args.typed_data)?;
    let computed = TypedDataDocument::parse_json(&json)?.compute()?;
    match args.format {
        DigestFormat::Text => render_computed_text(&computed, args.no_color)?,
        DigestFormat::Json => write_stdout(&serde_json::to_string_pretty(&computed)?)?,
    }
    Ok(ExitCode::SUCCESS)
}

fn examples(args: &ExamplesArgs) -> Result<ExitCode> {
    if args.json {
        write_stdout(EXAMPLES_MANIFEST)?;
        return Ok(ExitCode::SUCCESS);
    }
    let examples: Vec<ExampleSummary> =
        serde_json::from_str(EXAMPLES_MANIFEST).context("bundled example manifest is invalid")?;
    let mut output = String::new();
    for example in examples {
        let status = example.fault.as_deref().unwrap_or("control");
        writeln!(&mut output, "{}\t{}\t{}", example.id, status, example.title)
            .context("failed to format bundled examples")?;
    }
    write_stdout(output.trim_end())?;
    Ok(ExitCode::SUCCESS)
}

fn explain(args: &ExplainArgs) -> Result<ExitCode> {
    let normalized = args.code.to_ascii_uppercase().replace('-', "_");
    let code = FindingCode::ALL
        .iter()
        .copied()
        .find(|candidate| candidate.as_str() == normalized)
        .with_context(|| format!("unknown diagnostic code `{}`", args.code))?;
    let heading = format!("## {}", code.as_str());
    let start = DIAGNOSTICS
        .find(&heading)
        .with_context(|| format!("documentation is missing for {code}"))?;
    let section = &DIAGNOSTICS[start..];
    let end = section[heading.len()..]
        .find("\n## ")
        .map_or(section.len(), |offset| heading.len() + offset);
    write_stdout(section[..end].trim())?;
    Ok(ExitCode::SUCCESS)
}

fn read_input(path: &Path) -> Result<String> {
    if path.as_os_str() == OsStr::new("-") {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .context("failed to read typed data from stdin")?;
        return Ok(input);
    }
    fs::read_to_string(path)
        .with_context(|| format!("failed to read typed data from {}", path.display()))
}

fn write_stdout(value: &str) -> Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{value}").context("failed to write output")
}
