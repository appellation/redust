//! A simple Redis client.

/// Stream RESP.
pub mod codec;
/// Connect to Redis.
pub mod connection;
/// Redis models.
pub mod model;
/// Manage Redis connections with Deadpool.
pub mod pool;

pub use connection::Connection;
pub use pool::Manager;
pub use resp;

pub type Error = resp::Error<'static>;
pub type Result<T, E = Error> = std::result::Result<T, E>;
