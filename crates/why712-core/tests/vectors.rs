//! Cross-implementation EIP-712 hashing vectors.

use alloy_primitives::{b256, keccak256};
use why712_core::TypedDataDocument;

const MAIL: &str = include_str!("../fixtures/vectors/eip712-mail.json");
const ARRAYS_AND_BYTES: &str = include_str!("../fixtures/vectors/arrays-and-bytes.json");
const LARGE_SAFE_RUST_NUMBER: &str = r#"{
  "types": {
    "EIP712Domain": [],
    "Amount": [{ "name": "value", "type": "uint256" }]
  },
  "primaryType": "Amount",
  "domain": {},
  "message": { "value": 9007199254740993 }
}"#;
const LARGE_SAFE_RUST_STRING: &str = r#"{
  "types": {
    "EIP712Domain": [],
    "Amount": [{ "name": "value", "type": "uint256" }]
  },
  "primaryType": "Amount",
  "domain": {},
  "message": { "value": "9007199254740993" }
}"#;
const ABOVE_U64_NUMBER: &str = r#"{
  "types": {
    "EIP712Domain": [],
    "Amount": [{ "name": "value", "type": "uint256" }]
  },
  "primaryType": "Amount",
  "domain": {},
  "message": { "value": 18446744073709551616 }
}"#;
const ABOVE_U64_STRING: &str = r#"{
  "types": {
    "EIP712Domain": [],
    "Amount": [{ "name": "value", "type": "uint256" }]
  },
  "primaryType": "Amount",
  "domain": {},
  "message": { "value": "18446744073709551616" }
}"#;

#[test]
fn computes_the_canonical_eip712_mail_vector() -> why712_core::Result<()> {
    let document = TypedDataDocument::parse_json(MAIL)?;
    let values = document.compute()?;

    assert_eq!(
        values.encode_type,
        "Mail(Person from,Person to,string contents)Person(string name,address wallet)"
    );
    assert_eq!(
        *values.type_hash.as_b256(),
        keccak256(values.encode_type.as_bytes())
    );
    assert_eq!(values.encode_data.len(), 96);
    assert_eq!(
        *values.domain_separator.as_b256(),
        b256!("f2cee375fa42b42143804025fc449deafd50cc031ca257e0b194a650a912090f")
    );
    assert_eq!(
        *values.digest.as_b256(),
        b256!("be609aee343fb3c4b28e1df9e632fca64fcfaede20f02e86244efddf30957bd2")
    );
    Ok(())
}

#[test]
fn separates_json_syntax_from_typed_data_shape_errors() {
    assert!(matches!(
        TypedDataDocument::parse_json("{"),
        Err(why712_core::Error::TypedDataJson { .. })
    ));
    assert!(matches!(
        TypedDataDocument::parse_json("{}"),
        Err(why712_core::Error::TypedDataShape { .. })
    ));
}

#[test]
fn computes_nested_arrays_and_dynamic_bytes_like_viem() -> why712_core::Result<()> {
    let document = TypedDataDocument::parse_json(ARRAYS_AND_BYTES)?;
    let values = document.compute()?;

    assert_eq!(
        values.encode_type,
        "Batch(string title,Attachment[] attachments,uint256[] ids)Attachment(string name,bytes data)"
    );
    assert_eq!(
        *values.type_hash.as_b256(),
        b256!("02aefee3ee5461543f44be7c8844e025bab8fad50cf88734cc6249cb6bf30622")
    );
    assert_eq!(values.encode_data.len(), 96);
    assert_eq!(
        *values.struct_hash.as_b256(),
        b256!("2149540e43ac5474d5e5d447d64b97800bb3dbd6ccefbe63e1c9365932ca8e43")
    );
    assert_eq!(
        *values.domain_separator.as_b256(),
        b256!("65c0a9b48dbff1f16f875f2ca382668feda1d9cff27bf1eec52b46232dc12df1")
    );
    assert_eq!(
        *values.digest.as_b256(),
        b256!("c669c5801d3e3fb634fc85343807d7432d334886fcfc18bac2da899169a71bec")
    );
    Ok(())
}

#[test]
fn preserves_integer_literals_above_javascript_safe_range() -> why712_core::Result<()> {
    let number = TypedDataDocument::parse_json(LARGE_SAFE_RUST_NUMBER)?.compute()?;
    let string = TypedDataDocument::parse_json(LARGE_SAFE_RUST_STRING)?.compute()?;

    assert_eq!(number.digest, string.digest);
    Ok(())
}

#[test]
fn requires_strings_for_uint256_values_above_u64() -> why712_core::Result<()> {
    assert!(matches!(
        TypedDataDocument::parse_json(ABOVE_U64_NUMBER),
        Err(why712_core::Error::TypedDataShape { .. })
    ));

    let string = TypedDataDocument::parse_json(ABOVE_U64_STRING)?;
    assert!(string.compute().is_ok());
    Ok(())
}
