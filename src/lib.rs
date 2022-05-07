//! A simple Redis client built on [tokio].

/// Stream RESP.
mod codec;
/// Connect to Redis.
mod connection;
/// Redis models.
pub mod model;
/// Manage Redis connections with [deadpool].
pub mod pool;

pub use resp;

pub use codec::Codec;
pub use connection::Connection;

/// Static RESP error returned from [Connection](connection::Connection) or [Codec](codec::Codec).
pub type Error = resp::Error<'static>;
/// Result with an error type defaulting to [Error].
pub type Result<T, E = Error> = std::result::Result<T, E>;
