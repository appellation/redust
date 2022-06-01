use std::sync::RwLock;

use bytes::{Bytes, BytesMut};
use redust_resp::{from_data, Data};

use crate::{Connection, Result};

/// A Redis script.
///
/// `K` is the number of keys used by this script.
///
/// See [Redis documentation](https://redis.io/docs/manual/programmability/eval-intro/) for
/// details on how to write scripts.
///
/// Intended to initialized statically. Since [`Script::new`] is not `const`, use
/// [lazy_static](https://crates.io/crates/lazy_static) to initialize scripts.
#[derive(Debug)]
pub struct Script<const K: usize> {
	contents: Bytes,
	hash: RwLock<BytesMut>,
}

impl<const K: usize> Script<K> {
	/// Create a new script. `contents` is the body of the script.
	///
	/// Note: [`include_bytes`] can be used to load your scripts from separate files at compile time.
	pub fn new(contents: &'static [u8]) -> Self {
		Self {
			contents: Bytes::from_static(contents),
			hash: Default::default(),
		}
	}

	/// Create an [`Invocation`] for this script using the given connection.
	pub fn exec<'script, 'conn>(
		&'script self,
		connection: &'conn mut Connection,
	) -> Invocation<'script, 'conn, '_, K> {
		Invocation {
			connection,
			script: self,
			args: Vec::new(),
			keys: Vec::with_capacity(K),
		}
	}

	fn set_hash(&self, new: &[u8]) {
		let mut hash = self.hash.write().unwrap();
		hash.clear();
		hash.extend_from_slice(new);
	}

	/// Load this script into Redis. Once loaded, the SHA1 hash is stored and can be used by future
	/// invocations to reduce network traffic and improve performance.
	pub async fn load(&self, connection: &mut Connection) -> Result<Bytes> {
		let res = connection
			.cmd([b"script".as_slice(), b"load", &*self.contents])
			.await?;

		let hash: BytesMut = from_data::<serde_bytes::ByteBuf>(res)?
			.into_iter()
			.collect();

		self.set_hash(&*hash);
		Ok(hash.freeze())
	}

	/// Get the SHA1 hash of this script, loading it if necessary.
	pub async fn get_hash(&self, connection: &mut Connection) -> Result<Bytes> {
		let hash = self.hash.read().unwrap().clone();

		if hash.is_empty() {
			Ok(self.load(connection).await?)
		} else {
			Ok(hash.freeze())
		}
	}
}

/// A [`Script`] invocation.
///
/// Set keys and arguments using [`Invocation::keys`] and [`Invocation::args`].
pub struct Invocation<'script, 'conn, 'data, const K: usize> {
	connection: &'conn mut Connection,
	script: &'script Script<K>,
	args: Vec<&'data [u8]>,
	keys: Vec<&'data [u8]>,
}

impl<'data, const K: usize> Invocation<'_, '_, 'data, K> {
	/// Set the arguments to be passed to this script.
	pub fn args<I, B>(mut self, args: I) -> Self
	where
		I: IntoIterator<Item = &'data B>,
		B: 'data + AsRef<[u8]> + ?Sized,
	{
		self.args = args.into_iter().map(|b| b.as_ref()).collect();
		self
	}

	/// Set the keys to be passed to this script.
	pub fn keys<B>(mut self, keys: [&'data B; K]) -> Self
	where
		B: 'data + AsRef<[u8]> + ?Sized,
	{
		self.keys = keys.into_iter().map(|b| b.as_ref()).collect();
		self
	}

	/// Invoke the script.
	pub async fn invoke(self) -> Result<Data<'static>> {
		let hash = self.script.get_hash(self.connection).await?;

		let key_len = K.to_string().into_bytes();
		let mut cmd = Vec::with_capacity(3 + K + self.args.len());
		cmd.append(&mut vec![b"evalsha".as_slice(), &*hash, &key_len]);
		cmd.extend_from_slice(&self.keys);
		cmd.extend_from_slice(&self.args);

		self.connection.cmd(cmd).await
	}
}

#[cfg(test)]
mod test {
	use lazy_static::lazy_static;

	use crate::{connection::test::redis_url, Connection, Result};

	use super::Script;

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
		assert!(!TEST_SCRIPT.hash.read().unwrap().is_empty());

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
}
