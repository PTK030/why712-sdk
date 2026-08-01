//! Alloy HTTP adapter tests against a local JSON-RPC mock.

#![cfg(feature = "rpc")]
#![allow(clippy::unwrap_used)]

use alloy_primitives::{B256, Bytes, U256, address, b256};
use serde_json::json;
use why712_core::{ChainClient, rpc::AlloyChainClient};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_partial_json, method},
};

#[tokio::test]
async fn alloy_client_maps_the_three_required_json_rpc_methods() {
    let server = MockServer::start().await;
    mount_rpc(&server, "eth_chainId", json!("0x1"), 0).await;
    mount_rpc(&server, "eth_getCode", json!("0x6000"), 1).await;
    mount_rpc(&server, "eth_call", json!("0x1234"), 2).await;
    let client = AlloyChainClient::connect(&server.uri()).await.unwrap();
    let target = address!("00000000000000000000000000000000000000aa");

    assert_eq!(client.eth_chain_id().await.unwrap(), U256::from(1));
    assert_eq!(
        client.eth_get_code(target).await.unwrap(),
        Bytes::from_static(&[0x60, 0x00])
    );
    assert_eq!(
        client
            .eth_call(target, Bytes::from_static(&[0xde, 0xad]))
            .await
            .unwrap(),
        Bytes::from_static(&[0x12, 0x34])
    );
}

#[tokio::test]
async fn alloy_client_maps_historical_and_transaction_methods() {
    let server = MockServer::start().await;
    let hash: B256 = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let target = address!("00000000000000000000000000000000000000aa");
    let sender = address!("00000000000000000000000000000000000000bb");
    mount_rpc(
        &server,
        "eth_getTransactionByHash",
        json!({
            "hash": hash,
            "to": target,
            "from": sender,
            "input": "0xdeadbeef",
            "blockNumber": "0x64"
        }),
        0,
    )
    .await;
    mount_rpc(&server, "eth_getCode", json!("0x6000"), 1).await;
    mount_rpc(&server, "eth_call", json!("0x1234"), 2).await;
    let client = AlloyChainClient::connect(&server.uri()).await.unwrap();

    let transaction = client.eth_get_transaction_by_hash(hash).await.unwrap();
    assert_eq!(transaction.block_number, Some(100));
    assert_eq!(
        transaction.input,
        Bytes::from_static(&[0xde, 0xad, 0xbe, 0xef])
    );
    assert_eq!(
        client.eth_get_code_at(target, Some(99)).await.unwrap(),
        Bytes::from_static(&[0x60, 0x00])
    );
    assert_eq!(
        client
            .eth_call_from_at(target, sender, Bytes::from_static(&[0xde, 0xad]), Some(99),)
            .await
            .unwrap(),
        Bytes::from_static(&[0x12, 0x34])
    );
}

async fn mount_rpc(server: &MockServer, rpc_method: &str, result: serde_json::Value, id: u64) {
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "method": rpc_method })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })))
        .expect(1)
        .mount(server)
        .await;
}
