#[cfg(feature = "codec")]
pub use codec::Codec;
pub use error::{Error, Result};
pub use nom;
pub use resp2::data::{de::from_data, ser::to_data, Data};
pub use resp2::de::from_bytes;
pub use resp2::ser::to_bytes;

/// Stream RESP.
#[cfg(feature = "codec")]
mod codec;
mod de;
/// RESP errors.
mod error;
pub mod resp2;
/// Utils for RESP (de)serialization.
pub mod util;
