<p align="center"><img src="assets/logo.svg" width="72" height="72" alt="why712 标志"></p>

<h1 align="center">why712</h1>

<p align="center">
  <a href="README.md">English</a> · <a href="README.pl.md">Polski</a> · 简体中文 · <a href="README.ja.md">日本語</a>
</p>

<p align="center">由 Rust 引擎驱动的 TypeScript SDK，用于解释 EIP-712 签名为何无法验证。</p>

why712 是面向 EIP-712、EIP-2612、Permit2、ERC-1271 和 ERC-6492 的开发工具。诊断引擎
使用 Rust 编写，并以 WebAssembly 形式包含在 npm 包中。项目不提供托管应用或后端。

## TypeScript SDK

首次发布到 npm 后：

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

`diagnoseOffline()` 不执行任何 I/O。`diagnose()` 可以从开发者明确提供的 RPC 读取公开的
合约和协议状态。该包会为 Node.js 或浏览器自动选择正确的 WASM 加载器。

## 从源码构建

```bash
pnpm install --frozen-lockfile
pnpm build:sdk
pnpm check:sdk
pnpm test:sdk
```

配套 CLI 仍然保留：

```bash
cargo run -p why712 -- examples
cargo install --path crates/why712-cli --locked
```

why712 不会签名、不接收私钥、不提交交易，也不是钱包。完整且权威的内容请参阅
[英文 README](README.md) 和 [`docs/`](docs/)。

## 许可证

本项目依据 [MIT License](LICENSE) 开源。保留许可证声明后，可以使用、修改、分发、
再许可或销售软件副本。
