//! Browser bindings for the deterministic why712 diagnosis engine.

mod fetch;
mod input;

use serde::Serialize;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};
use why712_core::{
    TypedDataDocument, acquire_onchain_snapshot, acquire_protocol_snapshot,
    diagnose_offline as diagnose_core, is_erc6492_signature, probe_erc6492, protocol_request,
    render_markdown,
};

use crate::{fetch::FetchChainClient, input::WebRequest};

const EXAMPLES_MANIFEST: &str = include_str!("../../why712-core/fixtures/examples/manifest.json");

#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT_API: &str = r#"
export interface DiagnoseRequest {
  typedData: object | string;
  signature?: string;
  signer?: string;
  contract?: string;
  rpcUrl?: string;
  erc6492Validator?: string;
  chainId?: number;
}

export interface DiagnosticFinding {
  code: string;
  severity: "INFO" | "WARNING" | "ERROR";
  confidence: "CERTAIN" | "HIGH" | "HEURISTIC";
  message: string;
  expected?: string;
  actual?: string;
  remediation?: string;
  evidence?: string;
}

export interface DiagnosticReport {
  schemaVersion: 2;
  verdict: "MATCH" | "MISMATCH" | "INCONCLUSIVE";
  computed: Record<string, unknown>;
  expectedSigner?: string;
  recoveredSigner?: string;
  context?: {
    inputSource: "TYPED_DATA" | "WALLET_REQUEST" | "TRANSACTION";
    detectedStandard?: "EIP2612" | "PERMIT2_ALLOWANCE" | "PERMIT2_SIGNATURE_TRANSFER" | "ERC1271" | "ERC6492";
    chainId?: string;
    blockNumber?: number;
    blockTimestamp?: number;
    protocol?: Record<string, unknown>;
  };
  findings: DiagnosticFinding[];
}

export function diagnose_offline(input: DiagnoseRequest): DiagnosticReport;
export function diagnose(input: DiagnoseRequest): Promise<DiagnosticReport>;
export function report_to_markdown(report: DiagnosticReport): string;
export function examples(): unknown[];
"#;

/// Installs a panic hook that reports Rust panics to the browser console.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Runs deterministic diagnosis without making a network request.
///
/// # Errors
///
/// Returns a JavaScript error string when the request, typed data, or signature is malformed.
#[wasm_bindgen(skip_typescript)]
pub fn diagnose_offline(input: JsValue) -> Result<JsValue, JsValue> {
    let request = WebRequest::from_js(input)?;
    let report = diagnose_core(&request.into_core(None, None)?).map_err(js_error)?;
    to_js_object(&report)
}

/// Acquires optional chain evidence through browser `fetch`, then runs deterministic diagnosis.
///
/// RPC and CORS failures become report evidence and do not prevent offline diagnosis.
///
/// # Errors
///
/// Returns a JavaScript error string only for malformed local input or serialization failures.
#[wasm_bindgen(skip_typescript)]
pub async fn diagnose(input: JsValue) -> Result<JsValue, JsValue> {
    let request = WebRequest::from_js(input)?;
    let erc1271_input = request.erc1271_input()?;
    let (snapshot, failure) = match request.rpc_target()? {
        Some(target) => {
            let client = FetchChainClient::new(target.endpoint);
            match acquire_onchain_snapshot(&client, target.contract, target.signer, erc1271_input)
                .await
            {
                Ok(mut snapshot) => {
                    let typed_data = request.typed_data_json()?;
                    let document = TypedDataDocument::parse_json(&typed_data).map_err(js_error)?;
                    let protocol =
                        protocol_request(document.raw(), target.signer).map_err(js_error)?;
                    if let Some(protocol) = protocol {
                        snapshot.protocol = Some(
                            acquire_protocol_snapshot(
                                &client,
                                target.contract,
                                protocol,
                                request.signature(),
                                snapshot.block_number,
                                snapshot.block_timestamp,
                            )
                            .await,
                        );
                    }
                    if let (Some(validator), Some(signer), Some(signature)) = (
                        request.erc6492_validator()?,
                        target.signer,
                        request.signature(),
                    ) && is_erc6492_signature(signature)
                    {
                        let encoded = signature.strip_prefix("0x").unwrap_or(signature);
                        let bytes = alloy_primitives::hex::decode(encoded)
                            .map(alloy_primitives::Bytes::from)
                            .map_err(js_error)?;
                        let digest = document.compute().map_err(js_error)?.digest;
                        snapshot.erc6492 =
                            Some(probe_erc6492(&client, validator, signer, digest, bytes).await);
                    }
                    (Some(snapshot), None)
                }
                Err(error) => (None, Some(error.to_string())),
            }
        }
        None => (None, None),
    };
    let report = diagnose_core(&request.into_core(snapshot, failure)?).map_err(js_error)?;
    to_js_object(&report)
}

/// Renders a diagnostic report as GitHub-flavored Markdown.
///
/// # Errors
///
/// Returns a JavaScript error string when the report does not match the versioned schema.
#[wasm_bindgen(skip_typescript)]
pub fn report_to_markdown(input: JsValue) -> Result<String, JsValue> {
    let report = serde_wasm_bindgen::from_value(input)
        .map_err(|error| JsValue::from_str(&format!("invalid diagnostic report: {error}")))?;
    Ok(render_markdown(&report))
}

/// Returns the shared example manifest as native JavaScript values.
///
/// # Errors
///
/// Returns a JavaScript error string if the build-time manifest is invalid.
#[wasm_bindgen(skip_typescript)]
pub fn examples() -> Result<JsValue, JsValue> {
    let examples: serde_json::Value = serde_json::from_str(EXAMPLES_MANIFEST).map_err(js_error)?;
    to_js_object(&examples)
}

fn js_error(error: impl core::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn to_js_object(value: &impl Serialize) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true))
        .map_err(js_error)
}

#[cfg(test)]
mod tests {
    use why712_core::FindingCode;

    #[test]
    fn every_exported_finding_code_is_representable_as_a_string() {
        for code in FindingCode::ALL {
            assert!(!code.as_str().is_empty());
        }
    }
}
