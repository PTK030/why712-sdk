//! Cross-implementation EIP-712 hashing vectors.

use alloy_primitives::{b256, keccak256};
use why712_core::TypedDataDocument;

const MAIL: &str = include_str!("../fixtures/vectors/eip712-mail.json");

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
