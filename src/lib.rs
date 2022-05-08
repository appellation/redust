//! A simple Redis client & RESP parser for Rust.
//!
//! ```
//! use redis::{resp::{Data, from_data}, Connection};
//! # use redis::Error;
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
//! [Connection]s implement [TryStream](futures::TryStream) and [Sink](futures::Sink) for ergonomic
//! and idiomatic use. [deadpool] support is built-in to handle connection [pool]ing.
//!
//! Data is returned to the client as static [resp::Data]. The [resp] crate contains several
//! [serde] utilities for converting RESP into Rust structures. For reading data from a connection,
//! use [resp::from_data].
//!
//! Redis models are available in the [model] module. These are a convenient way to represent some
//! complex Redis responses in more ergonomic Rust structures, based on [serde].

/// Stream RESP.
mod codec;
/// Connect to Redis.
mod connection;
/// Redis models. `model` feature, default off.
#[cfg(feature = "model")]
pub mod model;
/// Manage Redis connections with [deadpool]. `pool` feature, default off.
#[cfg(feature = "pool")]
pub mod pool;

pub use resp;

pub use codec::Codec;
pub use connection::Connection;

/// Static [resp::Error] returned from [Connection] and [Codec].
pub type Error = resp::Error<'static>;
/// Result with an error type defaulting to [Error].
pub type Result<T, E = Error> = std::result::Result<T, E>;
