## What changed

<!-- One focused change. Explain the user-visible or architectural outcome. -->

## Why

<!-- Link the issue, EIP/ERC, pinned dependency source, or reproducible case. -->

## Documentation impact

<!-- List updated canonical/public docs, or write "None" and explain why. -->

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-features`
- [ ] `cargo check -p why712-core --target wasm32-unknown-unknown`
- [ ] New or changed behavior has a test.
- [ ] Every new `FindingCode` has a matching `docs/diagnostics.md` entry.
- [ ] Durable architecture, interoperability, or workflow knowledge is documented.
- [ ] Affected canonical, public, and translated docs are updated, or the documentation impact
      section explains why no update is needed.

## Safety

- [ ] No private key, seed phrase, RPC credential, or sensitive user data is included.
- [ ] Deterministic diagnosis remains free of network, filesystem, clock, and randomness.
- [ ] EIP-712 hashing still delegates to Alloy rather than a local implementation.

## Output changes

<!-- Paste reviewed report/snapshot diffs, or write "None". -->
