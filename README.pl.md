<p align="center"><img src="assets/logo.svg" width="320" height="116" alt="terminalowy wordmark why712"></p>

<h1 align="center">why712</h1>

<p align="center">
  <a href="README.md">English</a> · Polski · <a href="README.zh-CN.md">简体中文</a> · <a href="README.ja.md">日本語</a>
</p>

<p align="center">SDK TypeScript z silnikiem Rust, które wyjaśnia, dlaczego podpis EIP-712 nie przechodzi weryfikacji.</p>

why712 to narzędzie developerskie dla EIP-712, EIP-2612, Permit2, ERC-1271 i ERC-6492. Silnik
diagnostyczny jest napisany w Ruście i dostarczany w paczce npm jako WebAssembly. Nie ma
hostowanej aplikacji ani backendu why712.

## SDK TypeScript

Po pierwszym wydaniu npm:

```bash
pnpm add why712
```

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

`diagnoseOffline()` nie wykonuje żadnego I/O. `diagnose()` może odczytać publiczne dane kontraktu
i protokołu z endpointu RPC podanego przez developera. Paczka automatycznie wybiera właściwy
loader WASM dla Node.js albo przeglądarki.

## Budowanie ze źródeł

```bash
pnpm install --frozen-lockfile
pnpm build:sdk
pnpm check:sdk
pnpm test:sdk
```

CLI pozostaje narzędziem towarzyszącym:

```bash
cargo run -p why712 -- examples
cargo install --path crates/why712-cli --locked
```

why712 diagnozuje między innymi zły chain ID, domenę, kolejność pól, szerokości liczb, kodowanie
podpisu, nonce, deadline oraz wyniki ERC-1271/6492. Nie podpisuje wiadomości, nie przyjmuje kluczy
prywatnych i nie wysyła transakcji.

Pełna dokumentacja kanoniczna znajduje się w [angielskim README](README.md) oraz [`docs/`](docs/).

## Licencja

Projekt jest open source na [licencji MIT](LICENSE). Możesz go używać, modyfikować,
rozpowszechniać, sublicencjonować i sprzedawać z zachowaniem informacji licencyjnej.
