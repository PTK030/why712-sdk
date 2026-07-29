//! Fixture-backed tests for bounded hypothesis search.

use std::{fs, path::PathBuf};

use alloy_primitives::{Address, B256};
use why712_core::{
    FindingCode, HypothesisContext, HypothesisKind, MAX_CANDIDATES, ParsedSignature,
    TypedDataDocument, generate_candidates, search,
};

const MAIL_SIGNATURE: &str = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";
const ANVIL_SIGNER: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

#[test]
fn finds_wrong_chain_id_and_field_order() -> Result<(), Box<dyn std::error::Error>> {
    let signer: Address = "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826".parse()?;
    let signature = ParsedSignature::parse_hex(MAIL_SIGNATURE)?;

    let chain_hits = search(
        &load("wrong-chain-id.json")?,
        &signature,
        signer,
        &HypothesisContext::default(),
    );
    assert!(chain_hits.iter().any(|hit| {
        hit.code == FindingCode::DomainChainIdMismatch
            && hit.hypothesis == HypothesisKind::ChainId { value: 1 }
    }));

    let order_hits = search(
        &load("wrong-field-order.json")?,
        &signature,
        signer,
        &HypothesisContext::default(),
    );
    assert!(order_hits.iter().any(|hit| {
        hit.code == FindingCode::FieldOrderMismatch
            && hit.hypothesis
                == HypothesisKind::FieldOrder {
                    type_name: "Mail".to_owned(),
                    fields: vec!["from".to_owned(), "to".to_owned(), "contents".to_owned()],
                }
    }));
    Ok(())
}

#[test]
fn finds_permit2_domain_and_width_mutations() -> Result<(), Box<dyn std::error::Error>> {
    let signer: Address = ANVIL_SIGNER.parse()?;
    let signature = ParsedSignature::parse_hex(
        "0xf6c5e172da1e741538652e6ef9f7743c52155d596c8be6a1f1bd05c1487db44367124f7a079bada60d75f502417007b90122cd3b7e5581fa397ba60e2dca1c9d1b",
    )?;

    let version_hits = search(
        &load("permit2-version.json")?,
        &signature,
        signer,
        &HypothesisContext::default(),
    );
    assert!(version_hits.iter().any(|hit| {
        hit.hypothesis
            == HypothesisKind::RemoveDomainField {
                field: why712_core::DomainField::Version,
            }
    }));

    let width_hits = search(
        &load("width-mismatch.json")?,
        &signature,
        signer,
        &HypothesisContext::default(),
    );
    assert!(
        width_hits
            .iter()
            .any(|hit| hit.hypothesis == HypothesisKind::Permit2Widths)
    );
    Ok(())
}

#[test]
fn adds_a_known_domain_salt() -> Result<(), Box<dyn std::error::Error>> {
    let signer: Address = ANVIL_SIGNER.parse()?;
    let signature = ParsedSignature::parse_hex(
        "0xff5feaa9f213578935847cbf2d653307b8ec61d470b4ac84ba4b5237f1f41c82037b312fd8e5f7052a5ef53418966c1b1be5b9dd3120fb6511155dea58d7dc801b",
    )?;
    let salt: B256 =
        "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse()?;
    let context = HypothesisContext {
        domain_salts: vec![salt],
        ..HypothesisContext::default()
    };
    let hits = search(&load("missing-salt.json")?, &signature, signer, &context);

    assert!(hits.iter().any(|hit| {
        hit.hypothesis == HypothesisKind::AddDomainSalt { salt }
            && hit.code == FindingCode::DomainSaltMissing
    }));
    Ok(())
}

#[test]
fn generation_is_deterministic_ranked_and_bounded() -> Result<(), Box<dyn std::error::Error>> {
    let document = load("width-mismatch.json")?;
    let first = generate_candidates(&document, &HypothesisContext::default());
    let second = generate_candidates(&document, &HypothesisContext::default());

    assert_eq!(first, second);
    assert!(first.len() <= MAX_CANDIDATES);
    assert!(first.windows(2).all(|pair| pair[0].rank <= pair[1].rank));

    let fields: Vec<_> = (0..8)
        .map(|index| serde_json::json!({ "name": format!("field{index}"), "type": "uint256" }))
        .collect();
    let message: serde_json::Map<String, serde_json::Value> = (0..8)
        .map(|index| (format!("field{index}"), serde_json::Value::from(index)))
        .collect();
    let large = serde_json::json!({
        "types": {
            "EIP712Domain": [
                { "name": "name", "type": "string" },
                { "name": "chainId", "type": "uint256" }
            ],
            "Large": fields
        },
        "primaryType": "Large",
        "domain": { "name": "Bound test", "chainId": 1 },
        "message": message
    });
    let large = TypedDataDocument::parse_json(&serde_json::to_string(&large)?)?;
    assert_eq!(
        generate_candidates(&large, &HypothesisContext::default()).len(),
        MAX_CANDIDATES
    );
    Ok(())
}

fn load(name: &str) -> Result<TypedDataDocument, Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("examples")
        .join(name);
    Ok(TypedDataDocument::parse_json(&fs::read_to_string(path)?)?)
}
