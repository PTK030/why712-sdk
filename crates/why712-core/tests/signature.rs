//! Signature parsing and recovery vectors.

use alloy_primitives::{Address, U256, uint};
use why712_core::{Error, ParsedSignature, RecoveryEncoding, SignatureFormat, TypedDataDocument};

const MAIL: &str = include_str!("../fixtures/vectors/eip712-mail.json");
const MAIL_SIGNATURE: &str = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";
const SECP256K1_ORDER: U256 =
    uint!(0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141_U256);

#[test]
fn recovers_the_eip712_mail_signer() -> Result<(), Box<dyn std::error::Error>> {
    let digest = TypedDataDocument::parse_json(MAIL)?.compute()?.digest;
    let expected: Address = "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826".parse()?;
    let signature = ParsedSignature::parse_hex(MAIL_SIGNATURE)?;

    assert_eq!(signature.format(), SignatureFormat::Canonical);
    assert_eq!(
        signature.recovery(),
        RecoveryEncoding::Electrum { value: 28 }
    );
    assert!(!signature.has_zero_r());
    assert!(!signature.has_zero_s());
    assert!(!signature.has_high_s());
    assert_eq!(signature.recover_address(&digest), Some(expected));
    Ok(())
}

#[test]
fn classifies_v_without_hiding_transaction_style_values() -> Result<(), Box<dyn std::error::Error>>
{
    let digest = TypedDataDocument::parse_json(MAIL)?.compute()?.digest;
    let expected: Address = "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826".parse()?;
    let mut bytes = hex::decode(MAIL_SIGNATURE.trim_start_matches("0x"))?;

    bytes[64] = 1;
    let raw = ParsedSignature::parse_hex(&hex::encode(&bytes))?;
    assert_eq!(raw.recovery(), RecoveryEncoding::Raw { value: 1 });
    assert_eq!(raw.recover_address(&digest), Some(expected));

    bytes[64] = 38;
    let eip155 = ParsedSignature::parse_hex(&hex::encode(&bytes))?;
    assert_eq!(
        eip155.recovery(),
        RecoveryEncoding::Eip155 {
            value: 38,
            chain_id: 1
        }
    );
    assert_eq!(eip155.recover_address(&digest), Some(expected));

    bytes[64] = 2;
    let invalid = ParsedSignature::parse_hex(&hex::encode(&bytes))?;
    assert_eq!(invalid.recovery(), RecoveryEncoding::Invalid { value: 2 });
    assert_eq!(invalid.recover_address(&digest), None);
    Ok(())
}

#[test]
fn round_trips_the_erc2098_representation() -> Result<(), Box<dyn std::error::Error>> {
    let digest = TypedDataDocument::parse_json(MAIL)?.compute()?.digest;
    let canonical = ParsedSignature::parse_hex(MAIL_SIGNATURE)?;
    let compact_bytes = canonical
        .to_erc2098()
        .ok_or_else(|| std::io::Error::other("canonical signature has invalid v"))?;
    let compact = ParsedSignature::parse_hex(&hex::encode(compact_bytes))?;

    assert_eq!(compact.format(), SignatureFormat::Erc2098);
    assert_eq!(
        compact.recovery(),
        RecoveryEncoding::Compact { parity: true }
    );
    assert_eq!(
        compact.recover_address(&digest),
        canonical.recover_address(&digest)
    );
    Ok(())
}

#[test]
fn detects_high_s_without_rejecting_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let digest = TypedDataDocument::parse_json(MAIL)?.compute()?.digest;
    let canonical = ParsedSignature::parse_hex(MAIL_SIGNATURE)?;
    let mut bytes = hex::decode(MAIL_SIGNATURE.trim_start_matches("0x"))?;
    let high_s = SECP256K1_ORDER - canonical.s();
    bytes[32..64].copy_from_slice(&high_s.to_be_bytes::<32>());
    bytes[64] = 27;

    let malleable = ParsedSignature::parse_hex(&hex::encode(bytes))?;
    assert!(malleable.has_high_s());
    assert_eq!(
        malleable.recover_address(&digest),
        canonical.recover_address(&digest)
    );
    Ok(())
}

#[test]
fn keeps_invalid_scalars_as_diagnosable_state() -> why712_core::Result<()> {
    let mut bytes = [0_u8; 65];
    bytes[64] = 27;
    let signature = ParsedSignature::parse_hex(&hex::encode(bytes))?;
    let digest = TypedDataDocument::parse_json(MAIL)?.compute()?.digest;

    assert!(signature.has_zero_r());
    assert!(signature.has_zero_s());
    assert_eq!(signature.recover_address(&digest), None);
    Ok(())
}

#[test]
fn rejects_only_malformed_hex_and_lengths() {
    assert!(matches!(
        ParsedSignature::parse_hex("0xzz"),
        Err(Error::Hex {
            field: "signature",
            ..
        })
    ));
    assert!(matches!(
        ParsedSignature::parse_hex("0x00"),
        Err(Error::SignatureLength { len: 1 })
    ));
}
