//! End-to-end command, format, and exit-code contract tests.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

const MAIL_SIGNATURE: &str = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";
const MAIL_SIGNER: &str = "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826";

#[test]
fn check_uses_the_documented_exit_codes_and_formats() {
    command()
        .args([
            "check",
            "--typed-data",
            path("valid-mail.json").to_str().unwrap(),
            "--signature",
            MAIL_SIGNATURE,
            "--signer",
            MAIL_SIGNER,
            "--offline",
            "--no-color",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Verdict: MATCH"))
        .stdout(predicate::str::contains("digest:"));

    command()
        .args([
            "check",
            "--typed-data",
            path("wrong-chain-id.json").to_str().unwrap(),
            "--signature",
            MAIL_SIGNATURE,
            "--signer",
            MAIL_SIGNER,
            "--chain-id",
            "1",
            "--offline",
            "--format",
            "json",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"verdict\": \"MISMATCH\""))
        .stdout(predicate::str::contains("DOMAIN_CHAIN_ID_MISMATCH"));

    command()
        .args([
            "check",
            "--typed-data",
            path("wrong-chain-id.json").to_str().unwrap(),
            "--signature",
            MAIL_SIGNATURE,
            "--signer",
            MAIL_SIGNER,
            "--chain-id",
            "1",
            "--offline",
            "--format",
            "markdown",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::starts_with("# why712 diagnostic report"));
}

#[test]
fn digest_examples_and_explain_are_scriptable() {
    command()
        .args([
            "digest",
            "--typed-data",
            path("valid-mail.json").to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"digest\": \"0xbe609aee343fb3c4b28e1df9e632fca64fcfaede20f02e86244efddf30957bd2\"",
        ));

    command()
        .args(["examples"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wrong-chain-id"));

    command()
        .args(["explain", "domain-chain-id-mismatch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DOMAIN_CHAIN_ID_MISMATCH"))
        .stdout(predicate::str::contains("switch the connected chain"));
}

#[test]
fn usage_and_io_failures_exit_two() {
    command().args(["check"]).assert().code(2);

    command()
        .args(["digest", "--typed-data", "does-not-exist.json"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read typed data"));

    command()
        .args(["explain", "NOT_A_CODE"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown diagnostic code"));
}

#[test]
fn rpc_failure_is_visible_and_falls_back_offline() {
    command()
        .args([
            "check",
            "--typed-data",
            path("valid-mail.json").to_str().unwrap(),
            "--signature",
            MAIL_SIGNATURE,
            "--signer",
            MAIL_SIGNER,
            "--contract",
            "0x0000000000000000000000000000000000000001",
            "--rpc",
            "not-a-url",
            "--no-color",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "RPC enrichment failed; continuing offline",
        ))
        .stdout(predicate::str::contains("Verdict: MATCH"));

    command()
        .env("WHY712_RPC_URL", "not-a-url")
        .args([
            "check",
            "--typed-data",
            path("valid-mail.json").to_str().unwrap(),
            "--signature",
            MAIL_SIGNATURE,
            "--signer",
            MAIL_SIGNER,
            "--offline",
            "--no-color",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("Verdict: MATCH"));
}

#[test]
fn accepts_an_unmodified_wallet_request() {
    let typed_data = std::fs::read_to_string(path("valid-mail.json")).unwrap();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_signTypedData_v4",
        "params": [MAIL_SIGNER, typed_data]
    });
    command()
        .args([
            "check",
            "--wallet-request",
            "-",
            "--signature",
            MAIL_SIGNATURE,
            "--signer",
            MAIL_SIGNER,
            "--offline",
            "--format",
            "json",
        ])
        .write_stdin(request.to_string())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"inputSource\": \"WALLET_REQUEST\"",
        ));
}

fn command() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin("why712"))
}

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../why712-core/fixtures/examples")
        .join(name)
}
