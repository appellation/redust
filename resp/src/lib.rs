pub use data::{de::from_data, ser::to_data, Data};
pub use de::{from_bytes, Deserializer, ReadError};
pub use error::{Error, Result};
pub use nom;
pub use ser::{to_bytes, Serializer};

/// General form of RESP data.
mod data;
/// RESP deserialization.
mod de;
/// RESP errors.
mod error;
/// RESP parsing.
pub mod parser;
/// RESP serialization.
mod ser;
/// Utils for RESP (de)serialization.
pub mod util;
