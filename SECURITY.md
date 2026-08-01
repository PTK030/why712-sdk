# Security policy

## Reporting a vulnerability

Report exploitable vulnerabilities through
[GitHub private vulnerability reporting](https://github.com/PTK030/why712/security/advisories/new).
Do not open a public issue for an undisclosed vulnerability or include production keys, RPC
credentials, private typed data, or user signatures in a report.

Include the affected commit or version, impact, reproduction steps using synthetic data, and any
suggested mitigation. Maintainers will acknowledge a report when it is reviewed; no response-time
or bounty commitment is made during pre-release development.

## Supported versions

There is no stable release yet. Security fixes target the current `main` branch until published
versions are listed here.

## Trust and privacy boundary

why712 never needs a private key, seed phrase, wallet connection, or transaction approval. It
does not sign or submit anything.

Offline diagnosis performs no network, filesystem, clock, or randomness access after the caller
provides the typed-data input. The native CLI reads only the file or stdin path requested by the
user. There is no telemetry, account system, project backend, or RPC proxy.

When `--rpc` is used, the CLI contacts only the endpoint supplied explicitly or through
`WHY712_RPC_URL`. The endpoint can observe the caller's IP address and the public chain queries:
chain ID, transaction details requested by hash, block metadata, runtime code, domain getters,
permit state, and read-only verification calls. ERC-1271, EIP-2612/Permit2 simulation, and
optional ERC-6492 validation send the public digest
and signature as `eth_call` calldata. ERC-6492 validation uses only a validator address supplied
by the user, rejects wrappers above 128 KiB, and limits the call to five million gas. No call is
broadcast as a transaction.

RPC operators can return false data. Reports preserve the endpoint-derived values and probe
fallback source, but why712 does not establish chain consensus or endpoint integrity.

The TypeScript SDK has the same boundary. Typed data, signatures, and reports remain in the
calling Node.js or browser process. The embedded Rust/WebAssembly engine performs no implicit
telemetry or backend request. `diagnoseOffline()` performs no network I/O; `diagnose()` contacts
only the RPC URL explicitly supplied by the caller. Browser endpoints must permit CORS.

## Sensitive report content

Typed-data messages may contain orders, identities, nonces, or business data. JSON and Markdown
reports include hashes, signer addresses, findings, and portions of supplied or observed values.
Review output before pasting it into an issue, chat, or public log.

## Cryptographic scope

why712 diagnoses and recovers signatures; it is not a signer, wallet, key store, or formal
verification system. EIP-712 computation delegates to pinned Alloy libraries and is
cross-checked against independent vectors. A `MATCH` proves the supplied signature recovers the
expected EOA under the computed digest; it does not prove authorization, intent, contract state,
or transaction success.
