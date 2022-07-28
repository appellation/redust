use std::sync::Arc;

use futures::future::try_join_all;
use test_log::test;

use redust::{
	resp::{array, Data},
	Connection, Error, Result,
};
use tokio::{spawn, sync::Mutex};

use crate::common::redis_url;

mod common;

#[test(tokio::test)]
async fn ping() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;

	let res = conn.cmd(["PING"]).await?;
	assert_eq!(res, "PONG");

	Ok(())
}

#[test(tokio::test)]
async fn multi_ping() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;

	let res = conn.cmd(["PING"]).await?;
	assert_eq!(res, "PONG");

	let res = conn.cmd(["PING", "foobar"]).await?;
	assert_eq!(res, b"foobar");

	Ok(())
}

#[test(tokio::test)]
async fn stream() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;

	// return value is ID which is dynamic
	let res_id = conn.cmd(["XADD", "foo1", "*", "foo", "bar"]).await?;

	let res = conn.cmd(["XREAD", "STREAMS", "foo1", "0-0"]).await?;

	conn.cmd(["DEL", "foo1"]).await?;

	let expected = array![array![
		b"foo1",
		array![array![res_id, array![b"foo", b"bar"]]]
	]];

	assert_eq!(res, expected);
	Ok(())
}

#[test(tokio::test)]
async fn ping_stream() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;

	let cmds = [["ping", "foo"], ["ping", "bar"]];
	let res = conn.pipeline(cmds.iter()).await?;

	assert_eq!(
		res,
		vec![Data::bulk_string(b"foo"), Data::bulk_string(b"bar")]
	);

	Ok(())
}

// This cannot run in CI since debug commands are disabled
// #[tokio::test]
// async fn error() -> Result<()> {
// 	let mut conn = Connection::new(redis_url()).await?;

// 	let res = conn.cmd(["debug", "error", "uh oh"]).await;
// 	assert!(matches!(dbg!(res), Err(Error::Redis(msg)) if msg == "uh oh"));

// 	let res = conn.cmd(["ping"]).await?;
// 	assert_eq!(res, "PONG");

// 	Ok(())
// }

#[test(tokio::test)]
async fn many_sequential() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;

	for i in 0..1000 {
		let i_str = i.to_string();
		let res = conn.cmd(["PING", &i_str]).await?;
		assert!(matches!(res, Data::BulkString(i_bytes) if i_bytes == i_str.as_bytes()));
	}

	Ok(())
}

#[test(tokio::test)]
async fn many_parallel() -> Result<()> {
	let concurrency = 5;
	let conn = Arc::new(Mutex::new(Connection::new(redis_url()).await?));
	let mut futs = Vec::with_capacity(concurrency);

	for i in 0..concurrency {
		let conn2 = Arc::clone(&conn);
		let handle = spawn(async move {
			for j in (i * 1000)..(i * 1000 + 1000) {
				let j_str = j.to_string();
				let mut conn = conn2.lock().await;
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

#[cfg(feature = "command")]
#[test(tokio::test)]
async fn hello_no_auth() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;
	conn.run(redust::command::connection::Hello {
		username: None::<&str>,
		password: None::<&str>,
	})
	.await?;

	Ok(())
}

#[test(tokio::test)]
async fn blocking() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;
	let data = conn.cmd(["BLPOP", "empty", "5"]).await?;

	assert_eq!(data, ());
	Ok(())
}
