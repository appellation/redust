# redust benchmarks

Compares redust against [redis-rs](https://crates.io/crates/redis) and [fred](https://crates.io/crates/fred)
(and [redis-protocol](https://crates.io/crates/redis-protocol), fred's RESP parser) using
[criterion](https://crates.io/crates/criterion).

```sh
# in-memory RESP parsing/encoding, no server needed
cargo bench -p redust-benches --bench resp

# end-to-end client benchmarks, needs a running Redis
docker run --rm -d -p 6379:6379 redis
REDIS_URL=localhost:6379 cargo bench -p redust-benches --bench client
```

| Bench | Groups |
| --- | --- |
| `resp` | `parse` (simple string, integer, 1K/64K bulk strings, 100/1000 element arrays), `encode_set` |
| `client` | `ping`, `set_get` (16B/1K/64K values), `pipeline_set` (10/100/1000 commands) |

`redust` in `parse` is zero-copy (borrows from the input); `redust-owned` adds the `into_owned()`
the `Codec` does, which is the fairer comparison with the other parsers.

Each client benchmark uses a single connection (redis-rs's `MultiplexedConnection`, a fred
`Client`, a redust `Connection`) driven sequentially. HTML reports go to `target/criterion`.
