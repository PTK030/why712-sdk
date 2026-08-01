//! JavaScript contract tests for the generated WebAssembly bindings.
#![cfg(target_arch = "wasm32")]

use serde::Serialize;
use serde_json::{Value, json};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use why712_wasm::{diagnose, diagnose_offline, examples, report_to_markdown};

wasm_bindgen_test_configure!(run_in_browser);

const VALID_MAIL: &str = include_str!("../../why712-core/fixtures/examples/valid-mail.json");
const VALID_SIGNATURE: &str = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";
const VALID_SIGNER: &str = "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826";

fn valid_request() -> Value {
    json!({
        "typedData": serde_json::from_str::<Value>(VALID_MAIL).expect("fixture is valid JSON"),
        "signature": VALID_SIGNATURE,
        "signer": VALID_SIGNER
    })
}

fn request_to_js() -> wasm_bindgen::JsValue {
    valid_request()
        .serialize(&serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true))
        .expect("request serializes")
}

#[wasm_bindgen_test]
fn offline_binding_returns_the_native_match_verdict() {
    let output = diagnose_offline(request_to_js()).expect("diagnosis succeeds");
    let report: Value = serde_wasm_bindgen::from_value(output).expect("report deserializes");

    assert_eq!(report["schemaVersion"], 2);
    assert_eq!(report["verdict"], "MATCH");
    assert!(report["computed"]["digest"].is_string());
    assert!(report["findings"].as_array().is_some_and(Vec::is_empty));
}

#[wasm_bindgen_test(async)]
async fn async_binding_without_rpc_matches_offline_binding() {
    let output = diagnose(request_to_js()).await.expect("diagnosis succeeds");
    let report: Value = serde_wasm_bindgen::from_value(output).expect("report deserializes");

    assert_eq!(report["verdict"], "MATCH");
}

#[wasm_bindgen_test]
fn auxiliary_bindings_return_javascript_values() {
    let manifest: Value = serde_wasm_bindgen::from_value(examples().expect("manifest serializes"))
        .expect("manifest deserializes");
    let report = diagnose_offline(request_to_js()).expect("diagnosis succeeds");
    let markdown = report_to_markdown(report).expect("markdown renders");

    assert!(
        manifest
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["id"] == "valid-mail"))
    );
    assert!(markdown.contains("MATCH"));
    assert!(markdown.contains("EIP-712 computation"));
}
