pub fn redis_url() -> String {
	std::env::var("REDIS_URL").unwrap_or_else(|_| "localhost:6379".to_string())
}
