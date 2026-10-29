# Redust RESP

[![Redust](https://img.shields.io/crates/v/redust-resp)](https://crates.io/crates/redust-resp) [![Documentation](https://img.shields.io/docsrs/redust-resp)](https://docs.rs/redust-resp)

A simple RESP parser for Rust.

This crate implements [RESP2](https://redis.io/docs/reference/protocol-spec/), the protocol Redis
uses to communicate with clients, on top of [serde](https://serde.rs). It powers the
[redust](https://crates.io/crates/redust) Redis client but has no dependency on any particular
network stack.

- **serde integration**: deserialize RESP directly into your own types with `from_bytes`, or
  serialize them into RESP with `to_bytes`.
- **Dynamic data**: the `Data` enum represents any RESP value when the shape of a response isn't
  known ahead of time. Convert between `Data` and Rust types with `from_data` and `to_data`.
- **Zero-copy**: parsed strings borrow from the input wherever possible. Owned data from the
  codec shares the read buffer instead of copying out of it.
- **Errors as results**: RESP errors are returned as `Error::Redis` rather than as data.

## Usage

Deserialize RESP bytes into Rust types:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Reply<'a> {
    #[serde(borrow, with = "serde_bytes")]
    name: &'a [u8],
    count: i64,
}

let bytes = b"*4\r\n+name\r\n$5\r\nredis\r\n+count\r\n:42\r\n";
let (reply, remaining) = redust_resp::from_bytes::<Reply>(bytes).unwrap();

assert_eq!(reply, Reply { name: b"redis", count: 42 });
assert!(remaining.is_empty());
```

Or work with dynamic `Data`:

```rust
use redust_resp::{from_bytes, from_data, to_bytes, Data};

let (data, _) = from_bytes::<Data>(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n").unwrap();
assert_eq!(data, Data::Array(vec![Data::bulk_string("GET"), Data::bulk_string("foo")]));

let parts: Vec<&[u8]> = from_data(data.clone()).unwrap();
assert_eq!(parts, [b"GET", b"foo"]);

let mut buf = Vec::new();
to_bytes(&data, &mut buf).unwrap();
assert_eq!(buf, b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n");
```

The `util` module contains serde helpers for common Redis encodings, such as numbers stored in bulk
strings.

## Features

- `codec`: a [tokio-util](https://crates.io/crates/tokio-util) `Codec` which incrementally decodes
  RESP from a byte stream into `Data`.

## Benchmarks

In-memory comparison with the parser built into [redis-rs](https://crates.io/crates/redis) 1.8.0
and [redis-protocol](https://crates.io/crates/redis-protocol) 6.0.0 (the parser used by fred).
Times are criterion's median estimate; lower is better.

### Parsing into dynamic data

`redust` borrows from the input. `redust (owned)` also calls `Data::into_owned`, as the codec does,
which is the fairer comparison since the other parsers return owned values.

| Input | redust | redust (owned) | redis-rs | redis-protocol |
| --- | --: | --: | --: | --: |
| `+OK` | 18.5 ns | **34.1 ns** | 390 ns | 80.2 ns |
| `:1234567890` | 18.5 ns | **20.5 ns** | 429 ns | 58.7 ns |
| bulk string, 1 KiB | 14.3 ns | **91.0 ns** | 545 ns | 119 ns |
| bulk string, 64 KiB | 15.1 ns | **1.31 µs** | 8.51 µs | 1.49 µs |
| array of 100 × 16 B | 1.93 µs | 9.80 µs | 23.4 µs | **9.40 µs** |
| array of 1000 × 16 B | 18.8 µs | 90.9 µs | 217 µs | **87.5 µs** |

### Encoding a `SET` command

| Value size | redust | redis-rs | redis-protocol |
| --- | --: | --: | --: |
| 16 B | **210 ns** | 362 ns | 400 ns |
| 1 KiB | **219 ns** | 384 ns | 440 ns |
| 64 KiB | **1.21 µs** | 3.08 µs | 3.57 µs |

Measured on an AMD Ryzen 9 3900X running Windows 10 and Rust 1.95. To reproduce, see the
[`benches`](https://github.com/appellation/redust/tree/master/benches) crate.
