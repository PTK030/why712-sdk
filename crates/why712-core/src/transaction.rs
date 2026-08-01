//! Strict reconstruction of direct EIP-2612 and Permit2 verification transactions.

use alloy_primitives::{Address, B256, Bytes, U256, keccak256};
use alloy_sol_types::{SolCall, sol};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    ChainClient, ChainTransaction, DiagnosticInput, Error, InputSource, OnchainDomain, Result,
};

sol! {
    interface Eip2612Direct {
        function permit(
            address owner,
            address spender,
            uint256 value,
            uint256 deadline,
            uint8 v,
            bytes32 r,
            bytes32 s
        ) external;
    }
}

mod permit2_single {
    use alloy_sol_types::sol;
    sol! {
        struct PermitDetails {
            address token;
            uint160 amount;
            uint48 expiration;
            uint48 nonce;
        }
        struct PermitSingle {
            PermitDetails details;
            address spender;
            uint256 sigDeadline;
        }
        function permit(address owner, PermitSingle permitSingle, bytes signature) external;
    }
}

mod permit2_batch {
    use alloy_sol_types::sol;
    sol! {
        struct PermitDetails {
            address token;
            uint160 amount;
            uint48 expiration;
            uint48 nonce;
        }
        struct PermitBatch {
            PermitDetails[] details;
            address spender;
            uint256 sigDeadline;
        }
        function permit(address owner, PermitBatch permitBatch, bytes signature) external;
    }
}

mod signature_single {
    use alloy_sol_types::sol;
    sol! {
        struct TokenPermissions {
            address token;
            uint256 amount;
        }
        struct PermitTransferFrom {
            TokenPermissions permitted;
            uint256 nonce;
            uint256 deadline;
        }
        struct SignatureTransferDetails {
            address to;
            uint256 requestedAmount;
        }
        function permitTransferFrom(
            PermitTransferFrom permit,
            SignatureTransferDetails transferDetails,
            address owner,
            bytes signature
        ) external;
    }
}

mod signature_batch {
    use alloy_sol_types::sol;
    sol! {
        struct TokenPermissions {
            address token;
            uint256 amount;
        }
        struct PermitBatchTransferFrom {
            TokenPermissions[] permitted;
            uint256 nonce;
            uint256 deadline;
        }
        struct SignatureTransferDetails {
            address to;
            uint256 requestedAmount;
        }
        function permitTransferFrom(
            PermitBatchTransferFrom permit,
            SignatureTransferDetails[] transferDetails,
            address owner,
            bytes signature
        ) external;
    }
}

/// Exact execution call supplied by a recognized transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionCall {
    /// Transaction sender used as `msg.sender` during simulation.
    pub from: Address,
    /// Complete original calldata.
    pub calldata: Bytes,
}

/// Transaction evidence included in schema-v2 context.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionContext {
    /// Transaction hash.
    pub hash: B256,
    /// Direct verification target.
    pub target: Address,
    /// Original transaction sender.
    pub from: Address,
    /// Inclusion block, absent while pending.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_block: Option<u64>,
    /// Block used for state reads and simulation.
    pub state_block: u64,
    /// Whether the endpoint still reports the transaction as pending.
    pub pending: bool,
    /// Exact recognized operation.
    pub operation: String,
    /// Four-byte selector.
    pub selector: String,
}

/// Fully reconstructed input and execution evidence for one direct transaction.
#[derive(Clone, Debug)]
pub struct TransactionDiagnostic {
    /// Normalized EIP-712 document plus original RPC transaction JSON representation.
    pub input: DiagnosticInput,
    /// Signature decoded from calldata.
    pub signature: String,
    /// Signer/owner decoded from calldata.
    pub signer: Address,
    /// Direct verification contract.
    pub contract: Address,
    /// Exact call used for simulation.
    pub execution: ExecutionCall,
    /// State block timestamp.
    pub block_timestamp: u64,
    /// Report context.
    pub context: TransactionContext,
}

/// Fetches and strictly reconstructs one allowlisted direct verification transaction.
///
/// Mined transactions use their parent block. Pending transactions use the endpoint's latest
/// block. Unknown selectors and domains that cannot be reconstructed fail explicitly.
///
/// # Errors
///
/// Returns an error when the transaction is unavailable, its selector is not allowlisted, its
/// calldata is malformed, or required block/domain/state evidence cannot be read.
pub async fn reconstruct_transaction<C: ChainClient>(
    client: &C,
    hash: B256,
) -> Result<TransactionDiagnostic> {
    let transaction = client.eth_get_transaction_by_hash(hash).await?;
    let state_block = match transaction.block_number {
        Some(block) => block.checked_sub(1).ok_or_else(|| Error::Rpc {
            method: "eth_getTransactionByHash",
            reason: "a transaction in genesis has no parent state".to_owned(),
        })?,
        None => client.eth_block_number().await?,
    };
    let block_timestamp = client.eth_block_timestamp(state_block).await?;
    decode_transaction(client, transaction, state_block, block_timestamp).await
}

async fn decode_transaction<C: ChainClient>(
    client: &C,
    transaction: ChainTransaction,
    state_block: u64,
    block_timestamp: u64,
) -> Result<TransactionDiagnostic> {
    let selector = transaction
        .input
        .get(..4)
        .ok_or_else(|| unsupported(&transaction.input))?;
    if selector == Eip2612Direct::permitCall::SELECTOR {
        return reconstruct_eip2612(client, transaction, state_block, block_timestamp).await;
    }
    if selector == permit2_single::permitCall::SELECTOR {
        let call =
            permit2_single::permitCall::abi_decode(&transaction.input).map_err(calldata_error)?;
        let typed_data = permit2_allowance_json(
            transaction.to,
            client.eth_chain_id().await?,
            &call.permitSingle,
        );
        return finish(
            transaction,
            state_block,
            block_timestamp,
            "PERMIT2_PERMIT_SINGLE",
            &typed_data,
            call.signature,
            call.owner,
        );
    }
    if selector == permit2_batch::permitCall::SELECTOR {
        let call =
            permit2_batch::permitCall::abi_decode(&transaction.input).map_err(calldata_error)?;
        let typed_data = permit2_batch_json(
            transaction.to,
            client.eth_chain_id().await?,
            &call.permitBatch,
        );
        return finish(
            transaction,
            state_block,
            block_timestamp,
            "PERMIT2_PERMIT_BATCH",
            &typed_data,
            call.signature,
            call.owner,
        );
    }
    if selector == signature_single::permitTransferFromCall::SELECTOR {
        let call = signature_single::permitTransferFromCall::abi_decode(&transaction.input)
            .map_err(calldata_error)?;
        let typed_data = signature_transfer_json(
            transaction.to,
            client.eth_chain_id().await?,
            transaction.from,
            &call.permit,
        );
        return finish(
            transaction,
            state_block,
            block_timestamp,
            "PERMIT2_SIGNATURE_TRANSFER",
            &typed_data,
            call.signature,
            call.owner,
        );
    }
    if selector == signature_batch::permitTransferFromCall::SELECTOR {
        let call = signature_batch::permitTransferFromCall::abi_decode(&transaction.input)
            .map_err(calldata_error)?;
        let typed_data = signature_batch_json(
            transaction.to,
            client.eth_chain_id().await?,
            transaction.from,
            &call.permit,
        );
        return finish(
            transaction,
            state_block,
            block_timestamp,
            "PERMIT2_SIGNATURE_TRANSFER_BATCH",
            &typed_data,
            call.signature,
            call.owner,
        );
    }
    Err(unsupported(&transaction.input))
}

async fn reconstruct_eip2612<C: ChainClient>(
    client: &C,
    transaction: ChainTransaction,
    state_block: u64,
    block_timestamp: u64,
) -> Result<TransactionDiagnostic> {
    let call = Eip2612Direct::permitCall::abi_decode(&transaction.input).map_err(calldata_error)?;
    let domain_data = client
        .eth_call_at(
            transaction.to,
            Bytes::from(OnchainDomain::calldata()),
            Some(state_block),
        )
        .await?;
    let domain = OnchainDomain::decode(&domain_data)?;
    if !domain.extensions.is_empty() {
        return Err(Error::Rpc {
            method: "eip712Domain()",
            reason: "transaction reconstruction does not guess ERC-5267 extensions".to_owned(),
        });
    }
    let nonce = read_nonce(client, transaction.to, call.owner, state_block).await?;
    let typed_data = eip2612_json(&domain, &call, nonce)?;
    let mut signature = Vec::with_capacity(65);
    signature.extend_from_slice(call.r.as_slice());
    signature.extend_from_slice(call.s.as_slice());
    signature.push(call.v);
    finish(
        transaction,
        state_block,
        block_timestamp,
        "EIP2612_PERMIT",
        &typed_data,
        Bytes::from(signature),
        call.owner,
    )
}

fn finish(
    transaction: ChainTransaction,
    state_block: u64,
    block_timestamp: u64,
    operation: &str,
    typed_data: &Value,
    signature: Bytes,
    signer: Address,
) -> Result<TransactionDiagnostic> {
    let selector = format!("0x{}", hex::encode(&transaction.input[..4]));
    let original_json = transaction.original_json.clone();
    let typed_data_json =
        serde_json::to_string(typed_data).map_err(|error| Error::TypedDataShape {
            reason: error.to_string(),
        })?;
    let context = TransactionContext {
        hash: transaction.hash,
        target: transaction.to,
        from: transaction.from,
        transaction_block: transaction.block_number,
        state_block,
        pending: transaction.block_number.is_none(),
        operation: operation.to_owned(),
        selector,
    };
    Ok(TransactionDiagnostic {
        input: DiagnosticInput {
            typed_data_json,
            original_json,
            source: InputSource::Transaction,
        },
        signature: format!("0x{}", hex::encode(signature)),
        signer,
        contract: transaction.to,
        execution: ExecutionCall {
            from: transaction.from,
            calldata: transaction.input,
        },
        block_timestamp,
        context,
    })
}

async fn read_nonce<C: ChainClient>(
    client: &C,
    contract: Address,
    owner: Address,
    block: u64,
) -> Result<U256> {
    let mut calldata = Vec::with_capacity(36);
    calldata.extend_from_slice(&keccak256("nonces(address)".as_bytes())[..4]);
    calldata.extend_from_slice(&[0_u8; 12]);
    calldata.extend_from_slice(owner.as_slice());
    let result = client
        .eth_call_at(contract, Bytes::from(calldata), Some(block))
        .await?;
    result
        .get(..32)
        .map(U256::from_be_slice)
        .ok_or_else(|| Error::Rpc {
            method: "eth_call",
            reason: "nonces(address) result is shorter than one ABI word".to_owned(),
        })
}

fn eip2612_json(
    domain: &OnchainDomain,
    call: &Eip2612Direct::permitCall,
    nonce: U256,
) -> Result<Value> {
    let mut domain_types = Vec::new();
    let mut domain_value = serde_json::Map::new();
    if let Some(name) = &domain.name {
        domain_types.push(json!({"name":"name","type":"string"}));
        domain_value.insert("name".to_owned(), json!(name));
    }
    if let Some(version) = &domain.version {
        domain_types.push(json!({"name":"version","type":"string"}));
        domain_value.insert("version".to_owned(), json!(version));
    }
    if let Some(chain_id) = domain.chain_id {
        domain_types.push(json!({"name":"chainId","type":"uint256"}));
        domain_value.insert("chainId".to_owned(), json!(chain_id.to_string()));
    }
    if let Some(contract) = domain.verifying_contract {
        domain_types.push(json!({"name":"verifyingContract","type":"address"}));
        domain_value.insert("verifyingContract".to_owned(), json!(contract.to_string()));
    }
    if let Some(salt) = domain.salt {
        domain_types.push(json!({"name":"salt","type":"bytes32"}));
        domain_value.insert("salt".to_owned(), json!(salt.to_string()));
    }
    if domain_types.is_empty() {
        return Err(Error::Rpc {
            method: "eip712Domain()",
            reason: "ERC-5267 returned no reconstructable standard fields".to_owned(),
        });
    }
    Ok(json!({
        "types": {
            "EIP712Domain": domain_types,
            "Permit": [
                {"name":"owner","type":"address"},
                {"name":"spender","type":"address"},
                {"name":"value","type":"uint256"},
                {"name":"nonce","type":"uint256"},
                {"name":"deadline","type":"uint256"}
            ]
        },
        "primaryType": "Permit",
        "domain": domain_value,
        "message": {
            "owner": call.owner.to_string(),
            "spender": call.spender.to_string(),
            "value": call.value.to_string(),
            "nonce": nonce.to_string(),
            "deadline": call.deadline.to_string()
        }
    }))
}

fn permit2_domain(contract: Address, chain_id: U256) -> Value {
    json!({
        "name": "Permit2",
        "chainId": chain_id.to_string(),
        "verifyingContract": contract.to_string()
    })
}

fn permit2_types() -> Value {
    json!({
        "EIP712Domain": [
            {"name":"name","type":"string"},
            {"name":"chainId","type":"uint256"},
            {"name":"verifyingContract","type":"address"}
        ],
        "PermitDetails": [
            {"name":"token","type":"address"},
            {"name":"amount","type":"uint160"},
            {"name":"expiration","type":"uint48"},
            {"name":"nonce","type":"uint48"}
        ]
    })
}

fn permit2_allowance_json(
    contract: Address,
    chain_id: U256,
    permit: &permit2_single::PermitSingle,
) -> Value {
    let mut types = permit2_types();
    types["PermitSingle"] = json!([
        {"name":"details","type":"PermitDetails"},
        {"name":"spender","type":"address"},
        {"name":"sigDeadline","type":"uint256"}
    ]);
    json!({
        "types": types,
        "primaryType": "PermitSingle",
        "domain": permit2_domain(contract, chain_id),
        "message": {
            "details": detail_json(&permit.details),
            "spender": permit.spender.to_string(),
            "sigDeadline": permit.sigDeadline.to_string()
        }
    })
}

fn permit2_batch_json(
    contract: Address,
    chain_id: U256,
    permit: &permit2_batch::PermitBatch,
) -> Value {
    let mut types = permit2_types();
    types["PermitBatch"] = json!([
        {"name":"details","type":"PermitDetails[]"},
        {"name":"spender","type":"address"},
        {"name":"sigDeadline","type":"uint256"}
    ]);
    let details = permit
        .details
        .iter()
        .map(detail_batch_json)
        .collect::<Vec<_>>();
    json!({
        "types": types,
        "primaryType": "PermitBatch",
        "domain": permit2_domain(contract, chain_id),
        "message": {
            "details": details,
            "spender": permit.spender.to_string(),
            "sigDeadline": permit.sigDeadline.to_string()
        }
    })
}

fn detail_json(detail: &permit2_single::PermitDetails) -> Value {
    json!({
        "token": detail.token.to_string(),
        "amount": detail.amount.to_string(),
        "expiration": detail.expiration.to_string(),
        "nonce": detail.nonce.to_string()
    })
}

fn detail_batch_json(detail: &permit2_batch::PermitDetails) -> Value {
    json!({
        "token": detail.token.to_string(),
        "amount": detail.amount.to_string(),
        "expiration": detail.expiration.to_string(),
        "nonce": detail.nonce.to_string()
    })
}

fn signature_types() -> Value {
    json!({
        "EIP712Domain": [
            {"name":"name","type":"string"},
            {"name":"chainId","type":"uint256"},
            {"name":"verifyingContract","type":"address"}
        ],
        "TokenPermissions": [
            {"name":"token","type":"address"},
            {"name":"amount","type":"uint256"}
        ]
    })
}

fn signature_transfer_json(
    contract: Address,
    chain_id: U256,
    spender: Address,
    permit: &signature_single::PermitTransferFrom,
) -> Value {
    let mut types = signature_types();
    types["PermitTransferFrom"] = json!([
        {"name":"permitted","type":"TokenPermissions"},
        {"name":"spender","type":"address"},
        {"name":"nonce","type":"uint256"},
        {"name":"deadline","type":"uint256"}
    ]);
    json!({
        "types": types,
        "primaryType": "PermitTransferFrom",
        "domain": permit2_domain(contract, chain_id),
        "message": {
            "permitted": {
                "token": permit.permitted.token.to_string(),
                "amount": permit.permitted.amount.to_string()
            },
            "spender": spender.to_string(),
            "nonce": permit.nonce.to_string(),
            "deadline": permit.deadline.to_string()
        }
    })
}

fn signature_batch_json(
    contract: Address,
    chain_id: U256,
    spender: Address,
    permit: &signature_batch::PermitBatchTransferFrom,
) -> Value {
    let mut types = signature_types();
    types["PermitBatchTransferFrom"] = json!([
        {"name":"permitted","type":"TokenPermissions[]"},
        {"name":"spender","type":"address"},
        {"name":"nonce","type":"uint256"},
        {"name":"deadline","type":"uint256"}
    ]);
    let permitted = permit
        .permitted
        .iter()
        .map(|item| {
            json!({
                "token": item.token.to_string(),
                "amount": item.amount.to_string()
            })
        })
        .collect::<Vec<_>>();
    json!({
        "types": types,
        "primaryType": "PermitBatchTransferFrom",
        "domain": permit2_domain(contract, chain_id),
        "message": {
            "permitted": permitted,
            "spender": spender.to_string(),
            "nonce": permit.nonce.to_string(),
            "deadline": permit.deadline.to_string()
        }
    })
}

fn unsupported(input: &[u8]) -> Error {
    let selector = input.get(..4).map_or_else(
        || "missing".to_owned(),
        |bytes| format!("0x{}", hex::encode(bytes)),
    );
    Error::TypedDataShape {
        reason: format!(
            "transaction selector {selector} is not an allowlisted direct EIP-2612 or Permit2 call"
        ),
    }
}

fn calldata_error(error: impl core::fmt::Display) -> Error {
    Error::TypedDataShape {
        reason: format!("recognized transaction calldata is malformed: {error}"),
    }
}
