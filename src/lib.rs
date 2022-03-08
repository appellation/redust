//! A simple Redis client.

/// Items for streaming RESP.
pub mod codec;
/// Items for connecting to Redis.
pub mod connection;
/// Errors related to Redis interaction.
pub mod error;
/// Items for managing Redis connections with Deadpool.
pub mod pool;
/// Redis models.
pub mod model;

pub use connection::Connection;
pub use pool::Manager;
pub use resp;
