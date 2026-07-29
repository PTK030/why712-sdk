//! JavaScript request parsing and core input assembly.

use alloy_primitives::Address;
use serde::Deserialize;
use serde_json::Value;
use wasm_bindgen::JsValue;
use why712_core::{
    DiagnoseRequest, DomainProbe, Erc1271Input, Error, HypothesisContext, OnchainSnapshot,
    TypedDataDocument,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebRequest {
    typed_data: TypedDataInput,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    signer: Option<String>,
    #[serde(default)]
    contract: Option<String>,
    #[serde(default)]
    rpc_url: Option<String>,
    #[serde(default)]
    chain_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TypedDataInput {
    Json(String),
    Value(Value),
}

pub(crate) struct RpcTarget<'a> {
    pub(crate) endpoint: &'a str,
    pub(crate) contract: Address,
    pub(crate) signer: Option<Address>,
}

impl WebRequest {
    pub(crate) fn from_js(input: JsValue) -> Result<Self, JsValue> {
        serde_wasm_bindgen::from_value(input)
            .map_err(|error| JsValue::from_str(&format!("invalid diagnosis request: {error}")))
    }

    pub(crate) fn rpc_target(&self) -> Result<Option<RpcTarget<'_>>, JsValue> {
        match (&self.rpc_url, &self.contract) {
            (Some(_), None) => Err(JsValue::from_str("rpcUrl requires contract")),
            (None, None | Some(_)) => Ok(None),
            (Some(endpoint), Some(contract)) => Ok(Some(RpcTarget {
                endpoint,
                contract: parse_address(contract, "contract")?,
                signer: self
                    .signer
                    .as_deref()
                    .map(|value| parse_address(value, "signer"))
                    .transpose()?,
            })),
        }
    }

    pub(crate) fn erc1271_input(&self) -> Result<Option<Erc1271Input>, JsValue> {
        match (&self.signer, self.signature.as_deref()) {
            (Some(_), Some(signature)) => {
                let digest = TypedDataDocument::parse_json(&self.typed_data_json()?)
                    .and_then(|document| document.compute())
                    .map_err(|error| JsValue::from_str(&error.to_string()))?
                    .digest;
                Erc1271Input::from_signature_hex(digest, signature)
                    .map(Some)
                    .map_err(|error| JsValue::from_str(&error.to_string()))
            }
            _ => Ok(None),
        }
    }

    pub(crate) fn into_core(
        self,
        onchain_snapshot: Option<OnchainSnapshot>,
        onchain_failure: Option<String>,
    ) -> Result<DiagnoseRequest, JsValue> {
        let expected_signer = self
            .signer
            .as_deref()
            .map(|value| parse_address(value, "signer"))
            .transpose()?;
        let contract = self
            .contract
            .as_deref()
            .map(|value| parse_address(value, "contract"))
            .transpose()?;
        let mut hypothesis_context = HypothesisContext::default();
        if let Some(chain_id) = self.chain_id {
            hypothesis_context.chain_ids.push(chain_id);
        }
        if let Some(contract) = contract {
            hypothesis_context.verifying_contracts.push(contract);
        }
        if let Some(snapshot) = &onchain_snapshot {
            enrich_hypotheses(snapshot, &mut hypothesis_context);
        }
        let typed_data_json = self.typed_data_json()?;
        Ok(DiagnoseRequest {
            typed_data_json,
            signature: self.signature,
            expected_signer,
            hypothesis_context,
            onchain_snapshot,
            onchain_failure,
        })
    }

    fn typed_data_json(&self) -> Result<String, JsValue> {
        match &self.typed_data {
            TypedDataInput::Json(json) => Ok(json.clone()),
            TypedDataInput::Value(value) => {
                serde_json::to_string(value).map_err(|error| JsValue::from_str(&error.to_string()))
            }
        }
    }
}

fn enrich_hypotheses(snapshot: &OnchainSnapshot, context: &mut HypothesisContext) {
    if let Ok(chain_id) = u64::try_from(snapshot.chain_id) {
        context.chain_ids.push(chain_id);
    }
    context.verifying_contracts.push(snapshot.contract);
    if let DomainProbe::Erc5267 { domain, .. } = &snapshot.domain {
        if let Some(chain_id) = domain.chain_id.and_then(|value| u64::try_from(value).ok()) {
            context.chain_ids.push(chain_id);
        }
        if let Some(verifying_contract) = domain.verifying_contract {
            context.verifying_contracts.push(verifying_contract);
        }
        if let Some(salt) = domain.salt {
            context.domain_salts.push(salt);
        }
    }
}

fn parse_address(value: &str, field: &'static str) -> Result<Address, JsValue> {
    value
        .parse()
        .map_err(|_| JsValue::from_str(&Error::Address { field }.to_string()))
}
