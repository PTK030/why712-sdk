<p align="center"><img src="assets/logo.svg" width="320" height="116" alt="why712 ターミナルワードマーク"></p>

<h1 align="center">why712</h1>

<p align="center">
  <a href="README.md">English</a> · <a href="README.pl.md">Polski</a> · <a href="README.zh-CN.md">简体中文</a> · 日本語
</p>

<p align="center">EIP-712 署名が検証できない理由を説明する、Rust エンジン搭載の TypeScript SDK。</p>

why712 は EIP-712、EIP-2612、Permit2、ERC-1271、ERC-6492 向けの開発ツールです。
診断エンジンは Rust で実装され、WebAssembly として npm パッケージに同梱されます。
ホスト型アプリケーションや why712 バックエンドはありません。

## TypeScript SDK

最初の npm リリース後:

```bash
pnpm add why712
```

```ts
import { diagnoseOffline } from "why712";

const report = await diagnoseOffline({ typedData, signature, signer: expectedSigner });
for (const finding of report.findings) {
  console.error(finding.code, finding.remediation);
}
```

`diagnoseOffline()` は I/O を行いません。`diagnose()` は開発者が指定した RPC から
公開コントラクト／プロトコル情報を取得できます。パッケージは Node.js とブラウザーで
適切な WASM ローダーを自動選択します。

## ソースからビルド

```bash
pnpm install --frozen-lockfile
pnpm build:sdk
pnpm check:sdk
pnpm test:sdk
```

CLI も引き続き利用できます:

```bash
cargo run -p why712 -- examples
cargo install --path crates/why712-cli --locked
```

why712 は署名、秘密鍵の受け取り、トランザクション送信、ウォレット操作を行いません。
完全な正規ドキュメントは[英語 README](README.md) と [`docs/`](docs/) を参照してください。

## ライセンス

このプロジェクトは [MIT License](LICENSE) に基づくオープンソースです。ライセンス表示を
保持することで、使用、変更、配布、サブライセンス、販売が可能です。
