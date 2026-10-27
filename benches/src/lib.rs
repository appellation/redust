//! Shared helpers for the redust comparison benchmarks.

/// Redis address in `host:port` form, read from `REDIS_URL` (matching the integration tests).
pub fn redis_addr() -> String {
	let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "localhost:6379".to_string());
	match url.split_once("://") {
		Some((_, addr)) => addr.to_string(),
		None => url,
	}
}

/// Redis address as a `redis://` URL, for clients which require one.
pub fn redis_url() -> String {
	format!("redis://{}", redis_addr())
}

/// Build a multi-threaded Tokio runtime for async benchmarks.
pub fn runtime() -> tokio::runtime::Runtime {
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.expect("tokio runtime should build")
}
