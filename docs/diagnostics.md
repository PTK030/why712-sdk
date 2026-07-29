# Diagnostic catalog

`FindingCode` values are the stable contract shared by JSON output, the CLI, the web demo, and
markdown reports. Human wording may improve; these identifiers do not change casually.

## DOMAIN_SEPARATOR_MISMATCH

The locally computed domain separator differs from the contract or known signed domain.
Compare the individual domain fields; a more specific domain finding may identify the fix.

## DOMAIN_CHAIN_ID_MISMATCH

The signature verifies with another chain ID. Sign using the chain where the verifying contract
is deployed, or switch the connected chain before requesting the signature.

## DOMAIN_NAME_MISMATCH

The signature verifies with another domain name. Copy the exact name exposed or documented by
the verifying contract, including capitalization and whitespace.

## DOMAIN_VERSION_MISMATCH

The signature verifies with another domain version. Use the exact contract version rather than
assuming that all EIP-712 contracts use `"1"`.

## DOMAIN_VERSION_UNEXPECTED

The signature verifies after removing `version` from the domain. Omit the field; Permit2 is a
common example of a domain that has a name but no version.

## DOMAIN_VERIFYING_CONTRACT_MISMATCH

The signature verifies with another contract address. Use the actual verifier, accounting for
token, Permit2, proxy, and implementation addresses.

## DOMAIN_SALT_MISSING

The signed domain contains a salt omitted by the supplied typed data. Add the exact `bytes32`
salt and declare it in `EIP712Domain`.

## DOMAIN_SALT_MISMATCH

The supplied domain salt differs from the signed or on-chain salt. Copy the exact 32-byte value.

## TYPEHASH_MISMATCH

The primary type declaration does not match what was signed. Compare the type name, field names,
field types, nested dependencies, and their canonical ordering.

## FIELD_ORDER_MISMATCH

The signature verifies after reordering fields. EIP-712 struct field order is semantic; declare
fields in the same order used by the contract.

## ENCODE_PACKED_USED

The signature verifies when struct fields are ABI-packed. EIP-712 requires 32-byte encoded
members; replace `abi.encodePacked` with the contract's EIP-712 `abi.encode` construction.

## INTEGER_WIDTH_MISMATCH

The signature verifies with another integer width, such as Permit2's `uint160` or `uint48`
instead of `uint256`. Match the Solidity type exactly even when the current value fits both.

## MESSAGE_NUMERIC_PRECISION_LOSS

A JSON number exceeds JavaScript's safe integer range and may have been rounded before parsing.
Supply large integers as decimal strings and regenerate the signature from the exact value.

## SIG_V_NOT_NORMALIZED

The signature uses recovery parity `0`/`1` where the consumer expects historical `27`/`28`.
Convert only the final recovery byte by adding 27; do not change `r` or `s`.

## SIG_V_INVALID

The recovery byte is not a recognized raw, historical, or EIP-155 encoding. Obtain the original
signature again or correct the transport that altered its final byte.

## SIG_HIGH_S

The signature has a malleable high-S scalar. Normalize it to low-S and flip recovery parity, or
use a signing library that emits canonical low-S signatures.

## SIG_ZERO_R

The signature's `r` scalar is zero and cannot identify a valid ECDSA signature. Obtain a new,
unmodified signature.

## SIG_ZERO_S

The signature's `s` scalar is zero and cannot identify a valid ECDSA signature. Obtain a new,
unmodified signature.

## SIGNER_MISMATCH

The supplied digest and signature recover an address other than the expected signer. Resolve
the more specific domain, type, encoding, or signature findings before changing the signer.

## SIGNER_IS_CONTRACT

The expected signer has contract code and may validate through ERC-1271 rather than ECDSA
recovery. Call `isValidSignature(bytes32,bytes)` on that contract.

## ONCHAIN_DOMAIN_SELF_INCONSISTENT

The fields returned by ERC-5267 do not reproduce the contract's `DOMAIN_SEPARATOR()`. Treat the
contract or proxy configuration as inconsistent and inspect its domain implementation.

## ONCHAIN_DOMAIN_SOURCE

Records whether chain enrichment used structured ERC-5267 metadata, only the opaque
`DOMAIN_SEPARATOR()`, or neither probe. Failed optional probes remain in the evidence so a
fallback is explicit and reproducible rather than silently changing diagnostic confidence.

## BARE_STRUCT_HASH_SIGNED

The signature verifies over the primary `hashStruct` without the domain and `0x1901` envelope.
Sign the complete EIP-712 digest instead.

## EIP712_PREFIX_MISSING

The signature verifies after omitting the EIP-712 `0x1901` prefix. Include the prefix between
the domain separator and primary struct hash before the final Keccak-256.

## PERSONAL_SIGN_DETECTED

The signature verifies as EIP-191 `personal_sign` data rather than EIP-712 typed data. Request
typed-data signing from the wallet and pass the structured object, not serialized JSON text.

## UNHASHED_DYNAMIC_VALUE

The signature verifies when a dynamic `string` or `bytes` value is encoded directly. EIP-712
requires the Keccak-256 hash of dynamic values in the struct encoding.
