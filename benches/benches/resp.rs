//! RESP2 parsing and encoding benchmarks. These run entirely in memory and do not need a Redis
//! server.
//!
//! - `redust`: [`redust::resp`]
//! - `redis-rs`: the parser built into the `redis` crate
//! - `redis-protocol`: the parser used by `fred`

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use redis_protocol::resp2::{
	decode::decode,
	encode::encode,
	types::{OwnedFrame, Resp2Frame},
};
use redust::resp::{from_bytes, to_bytes, Data};

fn bulk_string(len: usize) -> Vec<u8> {
	let mut buf = format!("${len}\r\n").into_bytes();
	buf.resize(buf.len() + len, b'x');
	buf.extend_from_slice(b"\r\n");
	buf
}

fn bulk_array(count: usize, len: usize) -> Vec<u8> {
	let mut buf = format!("*{count}\r\n").into_bytes();
	for _ in 0..count {
		buf.extend(bulk_string(len));
	}
	buf
}

fn inputs() -> Vec<(&'static str, Vec<u8>)> {
	vec![
		("simple_string", b"+OK\r\n".to_vec()),
		("integer", b":1234567890\r\n".to_vec()),
		("bulk_string_1k", bulk_string(1024)),
		("bulk_string_64k", bulk_string(64 * 1024)),
		("array_100x16", bulk_array(100, 16)),
		("array_1000x16", bulk_array(1000, 16)),
	]
}

fn parse(c: &mut Criterion) {
	let mut group = c.benchmark_group("parse");

	for (name, input) in inputs() {
		group.throughput(Throughput::Bytes(input.len() as u64));

		group.bench_with_input(BenchmarkId::new("redust", name), &input, |b, input| {
			b.iter(|| black_box(from_bytes::<Data>(black_box(input)).unwrap()))
		});

		group.bench_with_input(
			BenchmarkId::new("redust-owned", name),
			&input,
			|b, input| {
				b.iter(|| {
					let (data, _) = from_bytes::<Data>(black_box(input)).unwrap();
					black_box(data.into_owned())
				})
			},
		);

		group.bench_with_input(BenchmarkId::new("redis-rs", name), &input, |b, input| {
			b.iter(|| black_box(redis::parse_redis_value(black_box(input)).unwrap()))
		});

		group.bench_with_input(
			BenchmarkId::new("redis-protocol", name),
			&input,
			|b, input| b.iter(|| black_box(decode(black_box(input)).unwrap().unwrap())),
		);
	}

	group.finish();
}

fn encode_cmd(c: &mut Criterion) {
	let mut group = c.benchmark_group("encode_set");

	for size in [16, 1024, 64 * 1024] {
		let key = b"redust-bench:key".as_slice();
		let value = vec![b'x'; size];
		group.throughput(Throughput::Bytes(size as u64));

		group.bench_with_input(BenchmarkId::new("redust", size), &value, |b, value| {
			let mut buf = Vec::with_capacity(size + 64);
			b.iter(|| {
				buf.clear();
				let data = Data::from_bytes_iter([b"SET".as_slice(), key, value]);
				to_bytes(&data, &mut buf).unwrap();
				black_box(&buf);
			})
		});

		group.bench_with_input(BenchmarkId::new("redis-rs", size), &value, |b, value| {
			b.iter(|| {
				black_box(
					redis::cmd("SET")
						.arg(key)
						.arg(value.as_slice())
						.get_packed_command(),
				)
			})
		});

		group.bench_with_input(
			BenchmarkId::new("redis-protocol", size),
			&value,
			|b, value| {
				let mut buf = Vec::with_capacity(size + 64);
				b.iter(|| {
					let frame = OwnedFrame::Array(vec![
						OwnedFrame::BulkString(b"SET".to_vec()),
						OwnedFrame::BulkString(key.to_vec()),
						OwnedFrame::BulkString(value.clone()),
					]);
					buf.clear();
					buf.resize(frame.encode_len(false), 0);
					encode(&mut buf, &frame, false).unwrap();
					black_box(&buf);
				})
			},
		);
	}

	group.finish();
}

criterion_group!(resp, parse, encode_cmd);
criterion_main!(resp);
