<p align="center"><img src="assets/logo.svg" width="88" height="88" alt="why712 标志"></p>

<h1 align="center">why712</h1>

<p align="center">
  <a href="README.md">English</a> · <a href="README.pl.md">Polski</a> · 简体中文 · <a href="README.ja.md">日本語</a>
</p>

<p align="center">本地签名恢复成功但合约拒绝时，找出具体原因。</p>

### [在浏览器中试用](https://ptk030.github.io/why712/) — 无需安装、无后端

why712 会计算全部 EIP-712 中间值、恢复签名者，并在一个有界集合中检查常见签名错误。
提供 RPC 端点时，它会通过 ERC-5267 或 `DOMAIN_SEPARATOR()` 读取合约的真实域，并明确
记录所使用的回退方式。
对于识别出的 EIP-2612 和 Permit2 消息，它还会比较合约 nonce 与区块时间。

- 诊断错误的 chain ID、域字段、字段顺序、整数宽度、紧凑编码、未哈希动态值、
  缺少 `0x1901`、裸结构哈希和 `personal_sign` 等问题。
- 支持标准 65 字节和 ERC-2098 紧凑签名，并检测错误 `v`、high-S 和零标量。
- 输出文本、版本化 JSON 或 GitHub Markdown。
- 可完全离线、确定性运行，无遥测、账户或后端。

## 从源码运行

项目通过 `rust-toolchain.toml` 固定 Rust 1.97。

```bash
cargo run -p why712 -- examples
cargo install --path crates/why712-cli --locked
```

## Docker 与浏览器演示

```bash
docker build --file docker/Dockerfile --tag why712:local .
docker run --rm --network none --read-only why712:local examples

pnpm install --frozen-lockfile
pnpm build
node scripts/serve-web.mjs
```

CLI 提供 `check`、`digest`、`examples` 和 `explain`。类型化数据可来自文件或标准输入
（`--typed-data -`）。`--offline` 始终禁止网络；RPC 模式同时需要 `--contract` 和
`--rpc`，也可通过 `WHY712_RPC_URL` 提供端点。

why712 不会签名、不接收私钥、不提交交易，也不是钱包。切勿提供私钥或助记词。

完整且权威的内容请参阅[英文 README](README.md)和 [`docs/`](docs/)。

## 许可证

当前版本依据 [PolyForm Noncommercial License 1.0.0](LICENSE) 以 source-available
方式提供：允许的非商业用途免费，商业用途需要单独付费许可。详情及联系方式见
[商业许可说明](COMMERCIAL-LICENSE.md)。
