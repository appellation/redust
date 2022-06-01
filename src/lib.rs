//! A simple Redis client & RESP parser for Rust.
//!
//! ```
//! use redust::{resp::{Data, from_data}, Connection};
//! # use redust::Error;
//!
//! # tokio_test::block_on(async {
//! let mut conn = Connection::new("localhost:6379").await?;
//! let res: Data = conn.cmd(["PING"]).await?;
//!
//! let deserialized: &str = from_data(res)?;
//! assert_eq!(deserialized, "PONG");
//! # Ok::<_, Error>(())
//! # });
//! ```
//!
//! [`Connection`]s implement [`TryStream`](futures::TryStream) and [`Sink`](futures::Sink) for ergonomic
//! and idiomatic use.
//!
//! Data is returned to the client as static [`resp::Data`]. The [resp] crate contains several
//! [serde] utilities for converting RESP into Rust structures. For reading data from a connection,
//! use [`resp::from_data`].
//!
//! # Additional Features
//!
//! - [`pool`]: connection pooling with [deadpool]
//! - [`model`]: complex Redis responses, based on [serde]
//! - [`script`]: Redis scripting utilities

/// Stream RESP.
mod codec;
/// Connect to Redis.
mod connection;

/// Redis models.
#[cfg(feature = "model")]
pub mod model;

/// Manage Redis connections with [deadpool].
///
/// ```rust
/// use redust::pool::{Manager, Pool};
///
/// # let _: () = {
/// let manager = Manager::new(([127, 0, 0, 1], 6379).into());
/// let pool = Pool::builder(manager).build().expect("pool should be built");
/// # };
/// ```
#[cfg(feature = "pool")]
pub mod pool;

/// Script utilities to handle SHA1 hash-based invocation.
///
/// ```rust
/// use redust::{resp::Data, script::Script, Connection};
/// # use redust::Error;
/// use lazy_static::lazy_static;
///
/// lazy_static! {
///     static ref MY_SCRIPT: Script<1> =
///         Script::new(b"return 'Hello ' .. redis.call('GET', KEYS[1]) .. ARGV[1]");
/// }
///
/// # tokio_test::block_on(async {
/// let mut conn = Connection::new("localhost:6379").await?;
/// conn.cmd(["SET", "hello", "world"]).await?;
///
/// let res: Data = MY_SCRIPT
///     .exec(&mut conn)
///     .args(["!"])
///     .keys(["hello"])
///     .invoke()
///     .await?;
///
/// assert_eq!(res, b"Hello world!");
/// # Ok::<_, Error>(())
/// # });
/// ```
#[cfg(feature = "script")]
pub mod script;

pub use redust_resp as resp;

pub use codec::Codec;
pub use connection::Connection;

/// Static [`resp::Error`] returned from [`Connection`] and [`Codec`].
pub type Error = resp::Error<'static>;
/// Result with an error type defaulting to [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;
