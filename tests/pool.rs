use futures::{future::try_join_all, Future};
use redust_resp::Data;
use test_log::test;

use redust::{
	pool::{Manager, Pool},
	Error, Result,
};
use tokio::spawn;

use crate::common::redis_url;

mod common;

fn assert_static<F>(_block: F)
where
	F: Future + Send + 'static,
{
}

#[test(tokio::test)]
async fn static_pool() -> Result<()> {
	let manager = Manager::new(redis_url());
	let pool = Pool::builder(manager).build().unwrap();

	assert_static(async move {
		let _ = pool.get().await;
	});

	Ok(())
}

#[test(tokio::test)]
async fn many_parallel() -> Result<()> {
	let concurrency = 1000;
	let iterations = 100;

	let manager = Manager::new(redis_url());
	let pool = Pool::builder(manager).build().unwrap();
	let mut futs = Vec::with_capacity(concurrency);

	for i in 0..concurrency {
		let pool = pool.clone();
		let handle = spawn(async move {
			for j in (i * iterations)..(i * iterations + iterations) {
				let j_str = j.to_string();
				let mut conn = pool.get().await.unwrap();
				let res = conn.cmd(["PING", &j_str]).await?;
				assert!(matches!(res, Data::BulkString(j_bytes) if j_bytes == j_str.as_bytes()));
			}

			Ok::<_, Error>(())
		});

		futs.push(handle);
	}

	try_join_all(futs)
		.await
		.unwrap()
		.into_iter()
		.for_each(|r| r.unwrap());
	Ok(())
}
