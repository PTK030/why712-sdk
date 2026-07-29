<p align="center"><img src="assets/logo.svg" width="88" height="88" alt="logo why712"></p>

<h1 align="center">why712</h1>

<p align="center">
  <a href="README.md">English</a> · Polski · <a href="README.zh-CN.md">简体中文</a> · <a href="README.ja.md">日本語</a>
</p>

<p align="center">Dowiedz się, dlaczego podpis EIP-712 nie przechodzi weryfikacji.</p>

### [Wypróbuj w przeglądarce](https://ptk030.github.io/why712/) — bez instalacji i backendu

why712 oblicza wszystkie wartości pośrednie EIP-712, odzyskuje adres podpisującego i przeszukuje
ograniczony zestaw typowych błędów podpisywania. Po podaniu endpointu RPC odczytuje prawdziwą
domenę kontraktu przez ERC-5267 lub `DOMAIN_SEPARATOR()` i dokładnie wskazuje użyty fallback.

- Diagnozuje m.in. zły chain ID, pola domeny, kolejność pól, szerokości liczb, `encodePacked`,
  niehaszowane wartości dynamiczne, brak `0x1901`, goły hash struktury i `personal_sign`.
- Obsługuje podpisy 65-bajtowe i ERC-2098 oraz wykrywa błędne `v`, high-S i zerowe skalary.
- Zwraca tekst, wersjonowany JSON albo Markdown zgodny z GitHubem.
- Działa deterministycznie offline, bez telemetrii, konta i backendu.

## Uruchomienie ze źródeł

Projekt przypina Rust 1.97 w `rust-toolchain.toml`.

```bash
cargo run -p why712 -- examples
cargo install --path crates/why712-cli --locked
```

Przykładowa diagnoza:

```bash
cargo run -p why712 -- check \
  --typed-data crates/why712-core/fixtures/examples/wrong-chain-id.json \
  --signature 0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c \
  --signer 0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826 \
  --chain-id 1 --offline
```

## Docker i demo

```bash
docker build --file docker/Dockerfile --tag why712:local .
docker run --rm --network none --read-only why712:local examples

pnpm install --frozen-lockfile
pnpm build
node scripts/serve-web.mjs
```

CLI udostępnia polecenia `check`, `digest`, `examples` i `explain`. Dane mogą pochodzić z pliku
lub stdin (`--typed-data -`). `--offline` zawsze blokuje sieć. Tryb RPC wymaga równocześnie
`--contract` i `--rpc`; adres może też pochodzić z `WHY712_RPC_URL`.

why712 nie podpisuje wiadomości, nie przyjmuje kluczy prywatnych, nie wysyła transakcji i nie
działa jako portfel. Nigdy nie podawaj klucza prywatnego ani frazy seed.

Pełna i kanoniczna dokumentacja znajduje się w [angielskim README](README.md) oraz w katalogu
[`docs/`](docs/).

## Licencja

Kod jest source-available na [PolyForm Noncommercial License 1.0.0](LICENSE): dozwolony użytek
niekomercyjny jest bezpłatny, a użytek komercyjny wymaga osobnej płatnej licencji. Szczegóły i
kontakt: [licencjonowanie komercyjne](COMMERCIAL-LICENSE.md).
