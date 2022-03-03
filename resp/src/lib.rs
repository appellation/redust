pub use nom;
use parser::{parse, Error};

pub mod parser;

const CRLF: [u8; 2] = [b'\r', b'\n'];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedData {
	SimpleString(String),
	Error(String),
	Integer(i64),
	BulkString(Option<Vec<u8>>),
	Array(Option<Vec<OwnedData>>),
}

impl<'a> From<Data<'a>> for OwnedData {
	fn from(other: Data<'a>) -> Self {
		match other {
			Data::SimpleString(str) => Self::SimpleString(str.to_owned()),
			Data::Error(str) => Self::Error(str.to_owned()),
			Data::Integer(int) => Self::Integer(int),
			Data::BulkString(str) => Self::BulkString(str.map(Vec::from)),
			Data::Array(data) => {
				Self::Array(data.map(|data| data.into_iter().map(OwnedData::from).collect()))
			}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Data<'a> {
	SimpleString(&'a str),
	Error(&'a str),
	Integer(i64),
	BulkString(Option<&'a [u8]>),
	Array(Option<Vec<Data<'a>>>),
}

impl<'a> TryFrom<&'a [u8]> for Data<'a> {
	type Error = Error<'a>;

	fn try_from(value: &'a [u8]) -> Result<Self, Error<'a>> {
		let (_, data) = parse(value)?;
		Ok(data)
	}
}

impl<'a> From<Data<'a>> for Vec<u8> {
	fn from(value: Data<'a>) -> Self {
		match value {
			Data::SimpleString(str) => {
				let mut data = Vec::with_capacity(str.len() + 3);
				data.push(b'+');
				data.extend_from_slice(str.as_bytes());
				data.extend_from_slice(&CRLF);
				data
			}
			Data::Error(str) => {
				let mut data = Vec::with_capacity(str.len() + 3);
				data.push(b'-');
				data.extend_from_slice(str.as_bytes());
				data.extend_from_slice(&CRLF);
				data
			}
			Data::Integer(int) => {
				let str = int.to_string();
				let mut data = Vec::with_capacity(str.len() + 3);
				data.push(b':');
				data.extend_from_slice(str.as_bytes());
				data.extend_from_slice(&CRLF);
				data
			}
			Data::BulkString(Some(bytes)) => {
				let len = bytes.len().to_string();
				let mut data = Vec::with_capacity(bytes.len() + len.len() + 5);
				data.push(b'$');
				data.extend_from_slice(len.as_bytes());
				data.extend_from_slice(&CRLF);
				data.extend_from_slice(bytes);
				data.extend_from_slice(&CRLF);
				data
			}
			Data::BulkString(None) => vec![b'$', b'-', b'1', b'\r', b'\n'],
			Data::Array(Some(data)) => {
				let mut out = vec![b'*'];

				let len = data.len().to_string();
				out.extend_from_slice(len.as_bytes());
				out.extend_from_slice(&CRLF);

				let bytes = data.into_iter().flat_map(Vec::<u8>::from);
				out.extend(bytes);
				out.extend_from_slice(&CRLF);

				out
			}
			Data::Array(None) => vec![b'*', b'-', b'1', b'\r', b'\n'],
		}
	}
}
