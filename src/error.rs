use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("IO error")]
	Io(#[from] ::std::io::Error),
	#[error("Parse error")]
	Parse,
}

pub type Result<T, E = Error> = ::std::result::Result<T, E>;
