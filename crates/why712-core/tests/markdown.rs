//! Markdown export snapshots for valid and diagnosed reports.

use alloy_primitives::Address;
use why712_core::{DiagnoseRequest, diagnose_offline, render_markdown};

const VALID_MAIL: &str = include_str!("../fixtures/examples/valid-mail.json");
const WRONG_CHAIN: &str = include_str!("../fixtures/examples/wrong-chain-id.json");
const SIGNATURE: &str = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";

#[test]
fn renders_match_and_root_cause_reports() -> Result<(), Box<dyn std::error::Error>> {
    let signer: Address = "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826".parse()?;
    let valid = diagnose_offline(&request(VALID_MAIL, signer))?;
    let wrong_chain = diagnose_offline(&request(WRONG_CHAIN, signer))?;

    insta::assert_snapshot!("markdown_match", render_markdown(&valid));
    insta::assert_snapshot!("markdown_root_cause", render_markdown(&wrong_chain));
    Ok(())
}

#[test]
fn escapes_values_that_can_break_tables() -> Result<(), Box<dyn std::error::Error>> {
    let signer: Address = "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826".parse()?;
    let mut report = diagnose_offline(&request(WRONG_CHAIN, signer))?;
    report.findings[0].expected = Some("left|right\nnext".to_owned());

    assert!(render_markdown(&report).contains("left\\|right<br>next"));
    Ok(())
}

fn request(typed_data_json: &str, signer: Address) -> DiagnoseRequest {
    DiagnoseRequest {
        typed_data_json: typed_data_json.to_owned(),
        signature: Some(SIGNATURE.to_owned()),
        expected_signer: Some(signer),
        ..DiagnoseRequest::default()
    }
}
