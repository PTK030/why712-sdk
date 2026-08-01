# why712 TypeScript SDK

TypeScript bindings for the Rust why712 diagnosis engine. The package runs locally through
WebAssembly in Node.js and modern browsers; it has no backend and never receives private keys.

```ts
import { diagnoseOffline } from "why712";

const report = await diagnoseOffline({
  typedData,
  signature,
  signer: expectedSigner,
});

for (const finding of report.findings) {
  console.error(finding.code, finding.remediation);
}
```

Use `diagnose({ ...input, rpcUrl, contract })` when contract and protocol evidence should be
read from an RPC endpoint. Browser RPC endpoints must permit CORS. Typed data and signatures
remain in the current JavaScript process except for the explicit RPC calls required to collect
public chain evidence.

Licensed under the MIT License; the package includes the complete license text.
