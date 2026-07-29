# Docker

The shipped image contains only the `why712` CLI and its runtime libraries. Rust, Cargo, a
shell, package managers, source code, and the browser application are absent from the final
distroless stage. The process runs as the image's `nonroot` user.

## Build

From the repository root:

```bash
docker build --file docker/Dockerfile --tag why712:local .
docker run --rm why712:local --help
```

The builder uses the pinned Rust 1.97 toolchain and `cargo build --locked`. The runtime is
`gcr.io/distroless/cc-debian12:nonroot`; its entrypoint uses JSON/vector form because distroless
does not contain a shell. The native RPC graph uses Rustls, not OpenSSL.

## Input and output contract

The image is a one-shot CLI:

- arguments after the image name are passed directly to `why712`;
- reports are written to standard output and warnings/errors to standard error;
- exit `0` means match, `1` means a diagnosed non-match or inconclusive result, and `2` means
  malformed input, usage, filesystem, or RPC setup failure;
- no writable filesystem is required;
- network access is needed only when `--rpc` is used;
- typed data can be mounted read-only or streamed through standard input.

Offline file input:

```bash
docker run --rm \
  --network none \
  --read-only \
  --volume "$PWD/permit.json:/input/permit.json:ro" \
  why712:local check \
  --typed-data /input/permit.json \
  --signature 0x... \
  --signer 0x... \
  --offline
```

Offline standard input:

```bash
docker run --rm --interactive --network none --read-only why712:local digest \
  --typed-data - < permit.json
```

RPC enrichment:

```bash
docker run --rm --read-only why712:local check \
  --typed-data /input/permit.json \
  --signature 0x... \
  --signer 0x... \
  --contract 0x... \
  --rpc https://your-endpoint.example
```

Mount `/input` read-only when using the RPC example with a file. Do not put RPC credentials in
the image or command history; pass a protected endpoint at runtime.

## Mainnet fork for repeatable probes

[`docker-compose.yml`](../docker-compose.yml) starts Anvil 1.7.1 on
`http://127.0.0.1:8545` and forks Ethereum mainnet through an upstream endpoint supplied at
runtime:

```bash
MAINNET_RPC_URL=https://your-endpoint.example docker compose up --wait anvil
```

Then point why712 at `http://host.docker.internal:8545` from another container, or
`http://127.0.0.1:8545` from the host. The diagnosed process talks only to the local fork.
Anvil lazily retrieves uncached mainnet state from `MAINNET_RPC_URL`, so the upstream endpoint
must remain available for addresses or storage slots not yet cached.

The compose service pins chain ID `1`, binds only to loopback, persists no chain state, and does
not deploy custom Solidity. Real mainnet USDC and Permit2 code/domain separators are therefore
available at their canonical addresses:

- USDC: `0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48`
- Permit2: `0x000000000022D473030F116dDEE9F6B43aC78BA3`

Stop and discard the fork with:

```bash
docker compose down
```
