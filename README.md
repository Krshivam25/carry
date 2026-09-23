# carry

Streams Hyperliquid and Lighter order books over WebSocket, maintains local books with sequence/nonce
continuity checks, normalizes funding to a common hourly/APR basis, and computes **executable**
delta-neutral carry for a given notional: funding spread − taker fees − slippage on both legs.

Read-only and paper trading only. It never handles private keys.

## Crates

| Crate | Kind | Purpose |
|---|---|---|
| `carry-core` | lib | Types, `OrderBook`, `FundingModel`, slippage walk. No async, no I/O. |
| `carry-venues` | lib | Serde message types, WS clients, reconnect/resync |
| `carry-sim` | lib | Two-leg position simulator, allocator |
| `carryd` | bin | Tokio tasks, axum API + SSE, sqlx recorder |


## Local gate (same as CI)

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```
