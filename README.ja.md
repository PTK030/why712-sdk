<p align="center"><img src="assets/logo.svg" width="88" height="88" alt="why712 ロゴ"></p>

<h1 align="center">why712</h1>

<p align="center">
  <a href="README.md">English</a> · <a href="README.pl.md">Polski</a> · <a href="README.zh-CN.md">简体中文</a> · 日本語
</p>

<p align="center">EIP-712 署名を検証できない理由を特定します。</p>

### [ブラウザーで試す](https://ptk030.github.io/why712/) — インストール不要、バックエンドなし

why712 は EIP-712 のすべての中間値を計算し、署名者を復元して、一般的な署名ミスを
有限の候補集合から探索します。RPC エンドポイントを指定すると、ERC-5267 または
`DOMAIN_SEPARATOR()` を使ってコントラクトの実際のドメインを読み取り、使用した
フォールバックを明示します。

- chain ID、ドメインフィールド、フィールド順、整数幅、packed encoding、
  ハッシュされていない動的値、`0x1901` の欠落、bare struct hash、
  `personal_sign` などを診断します。
- 65 バイト署名と ERC-2098 に対応し、不正な `v`、high-S、ゼロスカラーを検出します。
- テキスト、バージョン付き JSON、GitHub Markdown を出力します。
- テレメトリー、アカウント、バックエンドなしで決定論的にオフライン動作します。

## ソースから実行

Rust 1.97 は `rust-toolchain.toml` で固定されています。

```bash
cargo run -p why712 -- examples
cargo install --path crates/why712-cli --locked
```

## Docker とブラウザーデモ

```bash
docker build --file docker/Dockerfile --tag why712:local .
docker run --rm --network none --read-only why712:local examples

pnpm install --frozen-lockfile
pnpm build
node scripts/serve-web.mjs
```

CLI には `check`、`digest`、`examples`、`explain` があります。型付きデータはファイル
または標準入力（`--typed-data -`）から渡せます。`--offline` は常にネットワークを
無効化します。RPC モードには `--contract` と `--rpc` の両方が必要で、
`WHY712_RPC_URL` も利用できます。

why712 は署名、秘密鍵の受け取り、トランザクション送信、ウォレット操作を行いません。
秘密鍵やシードフレーズは絶対に入力しないでください。

完全な正規ドキュメントは[英語 README](README.md) と [`docs/`](docs/) を参照してください。

## ライセンス

現在のコードは [PolyForm Noncommercial License 1.0.0](LICENSE) に基づく
source-available です。許可された非商用利用は無料で、商用利用には別途有償ライセンス
が必要です。詳細と連絡方法は[商用ライセンス](COMMERCIAL-LICENSE.md)を参照してください。
