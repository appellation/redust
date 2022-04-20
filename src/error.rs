#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// An error occurred while performing IO.
	#[error("IO error")]
	Io(#[from] ::std::io::Error),
	/// An error occurred while parsing RESP from the server.
	#[error("Parse error")]
	Parse,
	#[error("Serialization error")]
	Serialize(#[from] resp::ser::Error),
}

pub type Result<T, E = Error> = ::std::result::Result<T, E>;
