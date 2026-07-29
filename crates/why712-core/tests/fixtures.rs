//! Contract tests for the fixture corpus shared by core, CLI, and the web demo.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use alloy_primitives::{Address, B256};
use serde::Deserialize;
use why712_core::{Digest, ParsedSignature, RecoveryEncoding, SignatureFormat, TypedDataDocument};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Example {
    id: String,
    title: String,
    typed_data: String,
    signature: Option<String>,
    signer: Option<String>,
    signed_digest: Option<String>,
    fault: Option<String>,
}

#[test]
fn example_manifest_is_self_contained_and_computable() -> Result<(), Box<dyn std::error::Error>> {
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("examples");
    let examples_root = examples_dir.canonicalize()?;
    let manifest = fs::read_to_string(examples_dir.join("manifest.json"))?;
    let examples: Vec<Example> = serde_json::from_str(&manifest)?;
    let mut ids = BTreeSet::new();
    let mut digests = BTreeMap::<String, Digest>::new();

    assert!(!examples.is_empty());
    for example in examples {
        assert!(ids.insert(example.id.clone()));
        assert!(!example.title.trim().is_empty());
        if let Some(fault) = &example.fault {
            assert!(!fault.trim().is_empty());
        }
        assert_eq!(example.signature.is_some(), example.signer.is_some());
        assert_eq!(example.signature.is_some(), example.signed_digest.is_some());
        if let (Some(signature), Some(signer), Some(signed_digest)) =
            (&example.signature, &example.signer, &example.signed_digest)
        {
            assert!(signature.starts_with("0x"));
            let signature = ParsedSignature::parse_hex(signature)?;
            let signer: Address = signer.parse()?;
            let signed_digest = Digest::from_b256(signed_digest.parse::<B256>()?);
            assert_eq!(signature.recover_address(&signed_digest), Some(signer));

            match example.id.as_str() {
                "wrong-chain-id" | "wrong-field-order" => {}
                "raw-v" => {
                    assert_eq!(signature.recovery(), RecoveryEncoding::Raw { value: 1 });
                    assert!(!signature.has_high_s());
                }
                "high-s" => assert!(signature.has_high_s()),
                "erc2098-mail" => {
                    assert_eq!(signature.format(), SignatureFormat::Erc2098);
                }
                _ => {
                    assert_eq!(signature.format(), SignatureFormat::Canonical);
                    assert!(!signature.has_high_s());
                }
            }

            let relative_path = Path::new(&example.typed_data);
            assert_eq!(relative_path.components().count(), 1);
            let typed_data_path = examples_dir.join(relative_path).canonicalize()?;
            assert!(typed_data_path.starts_with(&examples_root));
            let typed_data_json = fs::read_to_string(&typed_data_path)?;
            let values = TypedDataDocument::parse_json(&typed_data_json)?.compute()?;
            let recovered = signature.recover_address(&values.digest);
            if matches!(
                example.id.as_str(),
                "wrong-chain-id"
                    | "wrong-field-order"
                    | "encode-packed"
                    | "permit2-version"
                    | "missing-salt"
                    | "width-mismatch"
                    | "unhashed-dynamic"
            ) {
                assert_ne!(recovered, Some(signer));
            } else {
                assert_eq!(recovered, Some(signer));
            }
        }

        let relative_path = Path::new(&example.typed_data);
        assert_eq!(relative_path.components().count(), 1);
        let typed_data_path = examples_dir.join(relative_path).canonicalize()?;
        assert!(typed_data_path.starts_with(&examples_root));

        let typed_data_json = fs::read_to_string(typed_data_path)?;
        let values = TypedDataDocument::parse_json(&typed_data_json)?.compute()?;
        digests.insert(example.id, values.digest);
    }
    assert_ne!(digests.get("valid-mail"), digests.get("wrong-chain-id"));
    assert_ne!(digests.get("valid-mail"), digests.get("wrong-field-order"));
    Ok(())
}
