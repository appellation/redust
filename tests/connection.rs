use test_log::test;

use redust::{
	resp::{array, Data},
	Connection, Result,
};

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
