//! Direct diagnosis checks that do not require hypothesis search.

use alloy_primitives::{Address, Bytes, U256, address};
use why712_core::{
    DiagnoseRequest, DomainProbe, DomainSeparator, Error, FindingCode, OnchainSnapshot, Verdict,
    diagnose_offline,
};

const MAIL: &str = include_str!("../fixtures/examples/valid-mail.json");
const MAIL_SIGNATURE: &str = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";

#[test]
fn reports_invalid_recovery_and_zero_scalars() -> Result<(), Box<dyn std::error::Error>> {
    let signer: Address = "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826".parse()?;
    let mut invalid_v = hex::decode(MAIL_SIGNATURE.trim_start_matches("0x"))?;
    invalid_v[64] = 2;
    let invalid = diagnose_offline(&request(hex::encode(invalid_v), Some(signer)))?;
    assert_eq!(invalid.verdict, Verdict::Mismatch);
    assert_has_code(&invalid, FindingCode::SigVInvalid);

    let mut zero = [0_u8; 65];
    zero[64] = 27;
    let zero = diagnose_offline(&request(hex::encode(zero), Some(signer)))?;
    assert_has_code(&zero, FindingCode::SigZeroR);
    assert_has_code(&zero, FindingCode::SigZeroS);
    Ok(())
}

#[test]
fn supports_digest_only_and_rejects_malformed_signature() -> Result<(), Box<dyn std::error::Error>>
{
    let digest_only = diagnose_offline(&DiagnoseRequest {
        typed_data_json: MAIL.to_owned(),
        signature: None,
        expected_signer: None,
        ..DiagnoseRequest::default()
    })?;
    assert_eq!(digest_only.verdict, Verdict::Inconclusive);
    assert!(digest_only.findings.is_empty());

    let malformed = diagnose_offline(&request("0xzz".to_owned(), None));
    assert!(matches!(
        malformed,
        Err(Error::Hex {
            field: "signature",
            ..
        })
    ));
    Ok(())
}

#[test]
fn reports_onchain_separator_and_contract_signer_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let signer = address!("00000000000000000000000000000000000000bb");
    let mut request = request(MAIL_SIGNATURE.to_owned(), Some(signer));
    request.onchain_snapshot = Some(OnchainSnapshot {
        chain_id: U256::from(1),
        contract: address!("00000000000000000000000000000000000000aa"),
        contract_code: Bytes::from_static(&[0x60]),
        signer_code: Some(Bytes::from_static(&[0x60, 0x00])),
        domain: DomainProbe::SeparatorOnly {
            separator: DomainSeparator::from_b256([0x11; 32].into()),
        },
        failures: Vec::new(),
    });

    let report = diagnose_offline(&request)?;

    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert_has_code(&report, FindingCode::SignerIsContract);
    assert_has_code(&report, FindingCode::DomainSeparatorMismatch);
    assert!(
        !report
            .findings
            .iter()
            .any(|finding| finding.code == FindingCode::SignerMismatch)
    );
    Ok(())
}

fn request(signature: String, signer: Option<Address>) -> DiagnoseRequest {
    DiagnoseRequest {
        typed_data_json: MAIL.to_owned(),
        signature: Some(signature),
        expected_signer: signer,
        ..DiagnoseRequest::default()
    }
}

fn assert_has_code(report: &why712_core::Report, code: FindingCode) {
    assert!(report.findings.iter().any(|finding| finding.code == code));
}
