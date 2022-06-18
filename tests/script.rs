use lazy_static::lazy_static;

use redust::{script::Script, Connection, Result};

use crate::common::redis_url;

mod common;

lazy_static! {
	static ref TEST_SCRIPT: Script<0> = Script::new(b"return 'Hello world!'");
	static ref TEST_SCRIPT_ARG: Script<0> = Script::new(b"return 'Hello ' .. ARGV[1]");
	static ref TEST_SCRIPT_KEY: Script<1> =
		Script::new(b"return 'Hello ' .. redis.call('GET', KEYS[1])");
}

#[tokio::test]
async fn load_and_exec() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;

	let res = TEST_SCRIPT.exec(&mut conn).invoke().await?;
	assert_eq!(res, b"Hello world!");

	Ok(())
}

#[tokio::test]
async fn load_twice() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;

	dbg!(TEST_SCRIPT.load(&mut conn).await?);
	assert!(TEST_SCRIPT.is_loaded());

	let res = TEST_SCRIPT.exec(&mut conn).invoke().await?;
	assert_eq!(res, b"Hello world!");

	Ok(())
}

#[tokio::test]
async fn exec_with_arg() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;

	let res = TEST_SCRIPT_ARG
		.exec(&mut conn)
		.args(["world!"])
		.invoke()
		.await?;
	assert_eq!(res, b"Hello world!");

	Ok(())
}

#[tokio::test]
async fn exec_with_key() -> Result<()> {
	let mut conn = Connection::new(redis_url()).await?;

	conn.cmd(["set", "helloworld", "world!"]).await?;

	let res = TEST_SCRIPT_KEY
		.exec(&mut conn)
		.keys(["helloworld"])
		.invoke()
		.await?;
	assert_eq!(res, b"Hello world!");

	Ok(())
}
