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
