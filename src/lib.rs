//! A simple Redis client.

/// Items for connecting to Redis.
pub mod connection;
/// Items for managing Redis connections with Deadpool.
pub mod manager;
/// Redis models.
pub mod model;

pub use connection::Connection;
pub use manager::Manager;
pub use resp;
