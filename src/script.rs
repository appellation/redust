use std::sync::RwLock;

use bytes::{Bytes, BytesMut};
use redust_resp::{from_data, Data};
use tracing::instrument;

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
pub struct Script {
	contents: Bytes,
	hash: RwLock<BytesMut>,
}

impl Script {
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
	) -> Invocation<'script, 'conn, '_> {
		Invocation {
			connection,
			script: self,
			args: Vec::new(),
			keys: Vec::new(),
		}
	}

	/// Whether this script has been loaded.
	pub fn is_loaded(&self) -> bool {
		!self.hash.read().unwrap().is_empty()
	}

	fn set_hash(&self, new: &[u8]) {
		let mut hash = self.hash.write().unwrap();
		hash.clear();
		hash.extend_from_slice(new);
	}

	/// Load this script into Redis. Once loaded, the SHA1 hash is stored and can be used by future
	/// invocations to reduce network traffic and improve performance.
	#[instrument(level = "debug")]
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
	#[instrument(level = "trace")]
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
#[derive(Debug)]
pub struct Invocation<'script, 'conn, 'data> {
	connection: &'conn mut Connection,
	script: &'script Script,
	args: Vec<&'data [u8]>,
	keys: Vec<&'data [u8]>,
}

impl<'data> Invocation<'_, '_, 'data> {
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
	pub fn keys<I, B>(mut self, keys: I) -> Self
	where
		I: IntoIterator<Item = &'data B>,
		B: 'data + AsRef<[u8]> + ?Sized,
	{
		self.keys = keys.into_iter().map(|b| b.as_ref()).collect();
		self
	}

	/// Add an argument to be passed to this script. Prefer [`Script::args`] where possible.
	pub fn arg<B>(mut self, arg: &'data B) -> Self
	where
		B: AsRef<[u8]>,
	{
		self.args.push(arg.as_ref());
		self
	}

	/// Add a key to be passed to this script. Prefer [`Script::keys`] where possible.
	pub fn key<B>(mut self, key: &'data B) -> Self
	where
		B: AsRef<[u8]>,
	{
		self.keys.push(key.as_ref());
		self
	}

	/// Invoke the script.
	#[instrument(level = "debug")]
	pub async fn invoke(self) -> Result<Data<'static>> {
		let hash = self.script.get_hash(self.connection).await?;

		let key_len = self.keys.len().to_string().into_bytes();
		let mut cmd = Vec::with_capacity(3 + self.keys.len() + self.args.len());
		cmd.append(&mut vec![b"evalsha".as_slice(), &*hash, &key_len]);
		cmd.extend_from_slice(&self.keys);
		cmd.extend_from_slice(&self.args);

		self.connection.cmd(cmd).await
	}
}
