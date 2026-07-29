//! `why712` — command-line entry point.
//!
//! This binary is a thin shell over `why712-core`. It parses arguments, hands them to the
//! core, and renders the resulting report. No diagnostic logic lives here: anything the CLI
//! could tell you, the library and the WASM build must be able to tell you too.

use clap::{Parser, Subcommand};

/// Exit code returned when the diagnosis found something wrong with the signature.
///
/// Distinct from the usage/IO failure code, because this tool is meant to run in other
/// people's CI pipelines: "the signature is broken" and "you called me wrong" must not be
/// the same outcome.
const EXIT_MISMATCH: u8 = 1;

#[derive(Debug, Parser)]
#[command(name = "why712", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Diagnose a signature against typed data and, optionally, the verifying contract.
    Check,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Check => {
            // Wired up in M1 step 13, once the core reports something to render.
            let _ = EXIT_MISMATCH;
        }
    }
}
