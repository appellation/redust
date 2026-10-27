//! End-to-end client benchmarks against a live Redis server (`REDIS_URL`, default
//! `localhost:6379`).
//!
//! Every client is driven the same way: a single connection, timed with `iter_custom` so that
//! connection setup and any locking stay outside the measured loop.

use std::{
	hint::black_box,
	sync::Arc,
	time::{Duration, Instant},
};

use bytes::Bytes;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fred::prelude::{Builder, Client as FredClient, ClientLike, Config, KeysInterface};
use redis::aio::MultiplexedConnection;
use redust::Connection;
use redust_benches::{redis_addr, redis_url, runtime};
use tokio::{runtime::Runtime, sync::Mutex};

const KEY: &str = "redust-bench:key";
const VALUE_SIZES: [usize; 3] = [16, 1024, 64 * 1024];
const PIPELINE_SIZES: [usize; 3] = [10, 100, 1000];

struct Clients {
	redust: Arc<Mutex<Connection>>,
	redis: MultiplexedConnection,
	fred: FredClient,
}

impl Clients {
	async fn connect() -> Self {
		let redust = Connection::new(redis_addr())
			.await
			.expect("redust should connect");

		let redis = redis::Client::open(redis_url())
			.expect("redis-rs URL should be valid")
			.get_multiplexed_async_connection()
			.await
			.expect("redis-rs should connect");

		let fred =
			Builder::from_config(Config::from_url(&redis_url()).expect("fred URL should be valid"))
				.build()
				.expect("fred client should build");
		fred.init().await.expect("fred should connect");

		Self {
			redust: Arc::new(Mutex::new(redust)),
			redis,
			fred,
		}
	}
}

fn ping(c: &mut Criterion, rt: &Runtime, clients: &Clients) {
	let mut group = c.benchmark_group("ping");

	group.bench_function("redust", |b| {
		b.to_async(rt).iter_custom(|iters| {
			let conn = Arc::clone(&clients.redust);
			async move {
				let mut conn = conn.lock().await;
				let start = Instant::now();
				for _ in 0..iters {
					black_box(conn.cmd(["PING"]).await.unwrap());
				}
				start.elapsed()
			}
		})
	});

	group.bench_function("redis-rs", |b| {
		b.to_async(rt).iter_custom(|iters| {
			let mut conn = clients.redis.clone();
			async move {
				let start = Instant::now();
				for _ in 0..iters {
					let res: String = redis::cmd("PING").query_async(&mut conn).await.unwrap();
					black_box(res);
				}
				start.elapsed()
			}
		})
	});

	group.bench_function("fred", |b| {
		b.to_async(rt).iter_custom(|iters| {
			let client = clients.fred.clone();
			async move {
				let start = Instant::now();
				for _ in 0..iters {
					let res: String = client.ping(None).await.unwrap();
					black_box(res);
				}
				start.elapsed()
			}
		})
	});

	group.finish();
}

fn set_get(c: &mut Criterion, rt: &Runtime, clients: &Clients) {
	let mut group = c.benchmark_group("set_get");

	for size in VALUE_SIZES {
		let value = Bytes::from(vec![b'x'; size]);
		group.throughput(Throughput::Bytes(size as u64 * 2));

		group.bench_with_input(BenchmarkId::new("redust", size), &value, |b, value| {
			b.to_async(rt).iter_custom(|iters| {
				let conn = Arc::clone(&clients.redust);
				let value = value.clone();
				async move {
					let mut conn = conn.lock().await;
					let start = Instant::now();
					for _ in 0..iters {
						conn.cmd([b"SET".as_slice(), KEY.as_bytes(), &value])
							.await
							.unwrap();
						black_box(conn.cmd(["GET", KEY]).await.unwrap());
					}
					start.elapsed()
				}
			})
		});

		group.bench_with_input(BenchmarkId::new("redis-rs", size), &value, |b, value| {
			b.to_async(rt).iter_custom(|iters| {
				let mut conn = clients.redis.clone();
				let value = value.clone();
				async move {
					let start = Instant::now();
					for _ in 0..iters {
						let _: () = redis::cmd("SET")
							.arg(KEY)
							.arg(&value[..])
							.query_async(&mut conn)
							.await
							.unwrap();
						let res: Vec<u8> = redis::cmd("GET")
							.arg(KEY)
							.query_async(&mut conn)
							.await
							.unwrap();
						black_box(res);
					}
					start.elapsed()
				}
			})
		});

		group.bench_with_input(BenchmarkId::new("fred", size), &value, |b, value| {
			b.to_async(rt).iter_custom(|iters| {
				let client = clients.fred.clone();
				let value = value.clone();
				async move {
					let start = Instant::now();
					for _ in 0..iters {
						let _: () = client
							.set(KEY, value.clone(), None, None, false)
							.await
							.unwrap();
						let res: Bytes = client.get(KEY).await.unwrap();
						black_box(res);
					}
					start.elapsed()
				}
			})
		});
	}

	group.finish();
}

fn pipeline(c: &mut Criterion, rt: &Runtime, clients: &Clients) {
	// Every client builds its pipeline inside the timed loop, since fred pipelines are consumed
	// when executed.
	let mut group = c.benchmark_group("pipeline_set");

	for size in PIPELINE_SIZES {
		let keys: Vec<String> = (0..size).map(|i| format!("{KEY}:{i}")).collect();
		group.throughput(Throughput::Elements(size as u64));

		group.bench_with_input(BenchmarkId::new("redust", size), &keys, |b, keys| {
			b.to_async(rt).iter_custom(|iters| {
				let conn = Arc::clone(&clients.redust);
				async move {
					let mut conn = conn.lock().await;
					let start = Instant::now();
					for _ in 0..iters {
						let cmds = keys.iter().map(|k| ["SET", k.as_str(), "value"]);
						black_box(conn.pipeline(cmds).await.unwrap());
					}
					start.elapsed()
				}
			})
		});

		group.bench_with_input(BenchmarkId::new("redis-rs", size), &keys, |b, keys| {
			b.to_async(rt).iter_custom(|iters| {
				let mut conn = clients.redis.clone();
				async move {
					let start = Instant::now();
					for _ in 0..iters {
						let mut pipe = redis::pipe();
						for key in keys {
							pipe.set(key, "value");
						}
						let res: Vec<redis::Value> = pipe.query_async(&mut conn).await.unwrap();
						black_box(res);
					}
					start.elapsed()
				}
			})
		});

		group.bench_with_input(BenchmarkId::new("fred", size), &keys, |b, keys| {
			b.to_async(rt).iter_custom(|iters| {
				let client = clients.fred.clone();
				async move {
					let start = Instant::now();
					for _ in 0..iters {
						let pipe = client.pipeline();
						for key in keys {
							let _: () = pipe
								.set(key.as_str(), "value", None, None, false)
								.await
								.unwrap();
						}
						let res: Vec<fred::types::Value> = pipe.all().await.unwrap();
						black_box(res);
					}
					start.elapsed()
				}
			})
		});
	}

	group.finish();
}

fn benches(c: &mut Criterion) {
	let rt = runtime();
	let clients = rt.block_on(Clients::connect());

	ping(c, &rt, &clients);
	set_get(c, &rt, &clients);
	pipeline(c, &rt, &clients);

	rt.block_on(async {
		let mut conn = clients.redust.lock().await;
		let mut del = vec![KEY.to_string()];
		del.extend((0..PIPELINE_SIZES[PIPELINE_SIZES.len() - 1]).map(|i| format!("{KEY}:{i}")));
		conn.cmd(std::iter::once("DEL").chain(del.iter().map(String::as_str)))
			.await
			.unwrap();
	});
}

criterion_group! {
	name = client;
	config = Criterion::default().measurement_time(Duration::from_secs(5));
	targets = benches
}
criterion_main!(client);
