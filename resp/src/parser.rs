use std::str::from_utf8;

use nom::{
	branch::alt,
	bytes::streaming::take,
	character::streaming::{char, crlf, i64, not_line_ending},
	combinator::{map, map_res},
	error::ErrorKind,
	sequence::{delimited, terminated},
	IResult,
};

pub type Error<'a> = nom::Err<nom::error::Error<&'a [u8]>>;

pub fn parse_str(data: &[u8]) -> IResult<&[u8], &str> {
	map_res(delimited(char('+'), not_line_ending, crlf), from_utf8)(data)
}

pub fn parse_err(data: &[u8]) -> IResult<&[u8], &str> {
	map_res(delimited(char('-'), not_line_ending, crlf), from_utf8)(data)
}

pub fn parse_int(data: &[u8]) -> IResult<&[u8], i64> {
	delimited(char(':'), i64, crlf)(data)
}

pub fn parse_bytes(data: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
	let (data, len) = delimited(char('$'), i64, crlf)(data)?;
	Ok(match len {
		-1 => (data, None),
		0.. => map(terminated(take(len as usize), crlf), Some)(data)?,
		_ => Err(nom::Err::Failure(nom::error::Error::new(
			data,
			ErrorKind::Digit,
		)))?,
	})
}

pub fn parse_array(data: &[u8]) -> IResult<&[u8], i64> {
	delimited(char('*'), i64, crlf)(data)
}

pub fn parse_str_loose(data: &[u8]) -> IResult<&[u8], &str> {
	alt((
		parse_str,
		map_res(map(parse_bytes, Option::unwrap_or_default), from_utf8),
	))(data)
}

pub fn parse_int_loose(data: &[u8]) -> IResult<&[u8], i64> {
	alt((parse_int, map_res(parse_str_loose, str::parse)))(data)
}

// #[cfg(test)]
// mod test {
// 	use super::*;

// 	#[test]
// 	fn test_parse_str() {
// 		let resp = "OK\r\n".as_bytes();
// 		let (rem, res) = parse_str(resp).expect("Parsed string");

// 		assert_eq!(0, rem.len());
// 		assert_eq!("OK", res);
// 	}

// 	#[test]
// 	fn test_parse_int() {
// 		let resp = "10\r\n".as_bytes();
// 		let (rem, res) = parse_int(resp).expect("Parsed int");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(10, res);
// 	}

// 	#[test]
// 	fn test_parse_bytes() {
// 		let resp = "6\r\nfoobar\r\n".as_bytes();
// 		let (rem, res) = parse_bytes(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(Some("foobar".as_bytes()), res);
// 	}

// 	#[test]
// 	fn test_parse_empty_bytes() {
// 		let resp = "0\r\n\r\n".as_bytes();
// 		let (rem, res) = parse_bytes(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(Some([].as_slice()), res);
// 	}

// 	#[test]
// 	fn test_parse_null_bytes() {
// 		let resp = "-1\r\n".as_bytes();
// 		let (rem, res) = parse_bytes(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(None, res);
// 	}

// 	#[test]
// 	fn test_parse_array() {
// 		let resp = "2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_bytes();
// 		let (rem, res) = parse_array(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(
// 			Some(vec![
// 				Data::BulkString(Some(b"foo"[..].into())),
// 				Data::BulkString(Some(b"bar"[..].into()))
// 			]),
// 			res
// 		);
// 	}

// 	#[test]
// 	fn test_parse_empty_array() {
// 		let resp = "0\r\n".as_bytes();
// 		let (rem, res) = parse_array(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(Some(vec![]), res);
// 	}

// 	#[test]
// 	fn test_parse_null_array() {
// 		let resp = "-1\r\n".as_bytes();
// 		let (rem, res) = parse_array(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(None, res);
// 	}

// 	#[test]
// 	fn test_parse_resp_string() {
// 		let resp = "+OK\r\n".as_bytes();
// 		let (rem, res) = parse(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(Data::SimpleString("OK".into()), res);
// 	}

// 	#[test]
// 	fn test_parse_resp_error() {
// 		let resp = "-ERR unknown command 'foobar'\r\n".as_bytes();
// 		let (rem, res) = parse(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(Data::Error("ERR unknown command 'foobar'".into()), res);
// 	}

// 	#[test]
// 	fn test_parse_resp_int() {
// 		let resp = ":1000\r\n".as_bytes();
// 		let (rem, res) = parse(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(Data::Integer(1000), res);
// 	}

// 	#[test]
// 	fn test_parse_resp_bulk_string() {
// 		let resp = "$6\r\nfoobar\r\n".as_bytes();
// 		let (rem, res) = parse(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(Data::BulkString(Some(b"foobar"[..].into())), res);
// 	}

// 	#[test]
// 	fn test_parse_resp_array() {
// 		let resp = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_bytes();
// 		let (rem, res) = parse(resp).expect("Parsed bytes");

// 		assert_eq!(0, rem.len());
// 		assert_eq!(
// 			Data::Array(Some(vec![
// 				Data::BulkString(Some(b"foo"[..].into())),
// 				Data::BulkString(Some(b"bar"[..].into()))
// 			])),
// 			res
// 		);
// 	}
// }
