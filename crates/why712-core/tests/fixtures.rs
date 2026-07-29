//! Contract tests for the fixture corpus shared by core, CLI, and the web demo.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use alloy_primitives::Address;
use serde::Deserialize;
use why712_core::{Digest, TypedDataDocument};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Example {
    id: String,
    title: String,
    typed_data: String,
    signature: Option<String>,
    signer: Option<String>,
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
        if let Some(fault) = example.fault {
            assert!(!fault.trim().is_empty());
        }
        assert_eq!(example.signature.is_some(), example.signer.is_some());
        if let (Some(signature), Some(signer)) = (&example.signature, &example.signer) {
            assert!(signature.starts_with("0x"));
            assert_eq!(hex::decode(&signature[2..])?.len(), 65);
            let _: Address = signer.parse()?;
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
