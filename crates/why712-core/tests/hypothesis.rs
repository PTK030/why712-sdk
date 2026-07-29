//! Fixture-backed tests for bounded hypothesis search.

use std::{fs, path::PathBuf};

use alloy_primitives::{Address, B256};
use why712_core::{
    FindingCode, HypothesisContext, HypothesisKind, MAX_CANDIDATES, ParsedSignature,
    TypedDataDocument, generate_candidates, search,
};

const MAIL_SIGNATURE: &str = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";
const ANVIL_SIGNER: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
const VALID_MAIL: &str = include_str!("../fixtures/examples/valid-mail.json");

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

    let correct_contract: Address = "0xCcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC".parse()?;
    let wrong_contract = TypedDataDocument::parse_json(&VALID_MAIL.replace(
        "0xCcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC",
        "0x7777777777777777777777777777777777777777",
    ))?;
    let contract_hits = search(
        &wrong_contract,
        &signature,
        signer,
        &HypothesisContext {
            verifying_contracts: vec![correct_contract],
            ..HypothesisContext::default()
        },
    );
    assert!(contract_hits.iter().any(|hit| {
        hit.hypothesis
            == HypothesisKind::VerifyingContract {
                value: correct_contract,
            }
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
fn finds_alternate_encoding_and_envelope_strategies() -> Result<(), Box<dyn std::error::Error>> {
    let signer: Address = ANVIL_SIGNER.parse()?;
    let document = load("encode-packed.json")?;
    let cases = [
        (
            "0x6e8683e5274fd88ac427179b8258d4fe3b0ff1cef3e123a56a15b0daf2d113135e69c2740fb9add993181afc61f9baddff27fa339c4639446a4ea7f8b8feacb61c",
            HypothesisKind::EncodePacked,
            "0x71d31fd1a44c6681123136d7726e4f10baf874fc9d2f5ee77d41c29885c633d3",
        ),
        (
            "0xb10ec718e13a7974fdc950c1f97a4819fd5779ae26b3acb994343e24122bab3661460ffdfe2b0a16be46fb97ecb9dfee9c6b8f59814815e349df357b08315a781b",
            HypothesisKind::BareStructHash,
            "0xa21e55ea73f30e2f44684281052d13f505d1876037b8d257b349cec861046aff",
        ),
        (
            "0xf7c172fa3cabae1c4c2a3ab8ddc41579bc747be55da76ac154c1c00638b99a8c7e38339ef7d2a0a4f096c2361c4d47bcafad8e7891747871aecd9a81c14f6da01c",
            HypothesisKind::MissingEip712Prefix,
            "0xceebda57cff755fd6daebd3caa923cb18eb35e142d570e3c08892804a5d24b37",
        ),
        (
            "0xb71b1db4afaf6c965845b32a89e81971062aa7bade43ab8d3ba131b50a455a5377e820402bbaa5b06c47df43b212d247519e0fa19d23b1da8f02ee520f9066cd1c",
            HypothesisKind::PersonalSign,
            "0xdba6b87febac084370df9d377623ddbfd135d3304680042ac90b2d3e117e47c7",
        ),
    ];

    for (signature, hypothesis, digest) in cases {
        let hits = search(
            &document,
            &ParsedSignature::parse_hex(signature)?,
            signer,
            &HypothesisContext::default(),
        );
        let expected_digest: B256 = digest.parse()?;
        assert!(hits.iter().any(|hit| {
            hit.hypothesis == hypothesis && hit.digest.as_b256() == &expected_digest
        }));
    }
    Ok(())
}

#[test]
fn finds_unhashed_dynamic_values() -> Result<(), Box<dyn std::error::Error>> {
    let signer: Address = ANVIL_SIGNER.parse()?;
    let signature = ParsedSignature::parse_hex(
        "0x031d036c105fad4c7fb92b66e214f7fc72817183522bd1651a5aa10e3a12d21d1a2867061320a822a30c1356a21cc3fc44b11fbfa3da414152f361b11f5e5e241b",
    )?;
    let document = load("unhashed-dynamic.json")?;
    let actual = generate_candidates(&document, &HypothesisContext::default())
        .into_iter()
        .find(|candidate| candidate.hypothesis == HypothesisKind::UnhashedDynamicValue)
        .map(|candidate| candidate.digest);
    let hits = search(&document, &signature, signer, &HypothesisContext::default());
    let expected: B256 =
        "0x9c08459f5120ac358431a23027e0da0bf16c71f1f9ce0603ae30543063a473ef".parse()?;
    assert_eq!(actual.map(|digest| *digest.as_b256()), Some(expected));

    assert!(hits.iter().any(|hit| {
        hit.hypothesis == HypothesisKind::UnhashedDynamicValue && hit.digest.as_b256() == &expected
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
