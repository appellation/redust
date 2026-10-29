# Redust

[![Redust](https://img.shields.io/crates/v/redust)](https://crates.io/crates/redust) [![Documentation](https://img.shields.io/docsrs/redust)](https://docs.rs/redust)

A simple Redis client for Rust.

Redust is an async Redis client built on [tokio](https://tokio.rs). It speaks RESP using
[redust-resp](https://crates.io/crates/redust-resp), which is re-exported as `redust::resp`.

- **Simple**: send any command as a list of arguments and get back RESP `Data`.
- **Idiomatic**: a `Connection` is a `TryStream` and `Sink` of RESP data, so it works with the
  [futures](https://crates.io/crates/futures) combinators you already know, including for PubSub.
- **serde-powered**: deserialize responses into your own types with `resp::from_data`.
- **Pipelining**: send many commands at once with `Connection::pipeline`.

## Usage

```rust
use redust::{resp::{from_data, Data}, Connection};

#[tokio::main]
async fn main() -> redust::Result<()> {
    let mut conn = Connection::new("localhost:6379").await?;

    conn.cmd(["SET", "hello", "world"]).await?;
    let res: Data = conn.cmd(["GET", "hello"]).await?;

    let value: String = from_data(res)?;
    assert_eq!(value, "world");

    let results = conn.pipeline([["INCR", "count"], ["INCR", "count"]]).await?;
    assert_eq!(results.len(), 2);

    Ok(())
}
```

## Features

All features are disabled by default.

- `command`: type-safe commands, such as `Hello` and PubSub commands
- `model`: serde models for complex Redis responses, such as PubSub messages and stream IDs
- `pool`: connection pooling with [deadpool](https://crates.io/crates/deadpool)
- `script`: Lua scripts invoked by their SHA1 hash, loading them as necessary

## Benchmarks

End-to-end comparison with [redis-rs](https://crates.io/crates/redis) 1.8.0 and
[fred](https://crates.io/crates/fred) 10.1.0, each using a single connection driven sequentially.
Times are criterion's median estimate per iteration; lower is better.

| Benchmark | redust | redis-rs | fred |
| --- | --: | --: | --: |
| `PING` | **307 µs** | 317 µs | 324 µs |
| `SET` + `GET`, 16 B | **622 µs** | 632 µs | 651 µs |
| `SET` + `GET`, 1 KiB | **615 µs** | 647 µs | 642 µs |
| `SET` + `GET`, 64 KiB | 44.0 ms | 44.0 ms | 44.1 ms |
| pipeline, 10 × `SET` | **316 µs** | 331 µs | 523 µs |
| pipeline, 100 × `SET` | **441 µs** | 498 µs | 1.29 ms |
| pipeline, 1000 × `SET` | **1.30 ms** | 1.74 ms | 8.17 ms |

Single commands are dominated by network round trips, so all three clients are within a few
percent of each other; the 64 KiB case is bound by TCP delayed-ACK stalls (~40 ms) for every
client. Pipelining is where client overhead shows: redust is about 25% faster than redis-rs and
6× faster than fred at 1000 commands.

Measured on an AMD Ryzen 9 3900X running Windows 10 and Rust 1.95, against Redis 8.10.3 in Docker
Desktop on the same machine. Benchmarks for RESP parsing and encoding are in the
[redust-resp README](https://github.com/appellation/redust/tree/master/resp). To reproduce, see
the [`benches`](https://github.com/appellation/redust/tree/master/benches) crate.
