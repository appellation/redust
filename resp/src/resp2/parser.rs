//! RESP parsers.
//!
//! Each parser takes a cursor over the input and advances it past the parsed value on success. On
//! error, the cursor is left where it was.

use std::str::from_utf8;

use crate::{Needed, ParseError};

type Result<T> = std::result::Result<T, ParseError>;

/// Consume `tag`.
#[inline]
fn tag(input: &mut &[u8], tag: u8) -> Result<()> {
	match input.split_first() {
		Some((&b, rest)) if b == tag => {
			*input = rest;
			Ok(())
		}
		Some((&found, _)) => Err(ParseError::Unexpected {
			expected: tag,
			found,
		}),
		None => Err(ParseError::Incomplete(Needed::Size(1))),
	}
}

/// Consume CRLF.
#[inline]
fn crlf(input: &mut &[u8]) -> Result<()> {
	match *input {
		[b'\r', b'\n', rest @ ..] => {
			*input = rest;
			Ok(())
		}
		[] | [b'\r'] => Err(ParseError::Incomplete(Needed::Size(2 - input.len()))),
		_ => Err(ParseError::Crlf),
	}
}

/// Consume everything up to and including the next CRLF, returning the line without the CRLF.
#[inline]
fn line<'a>(input: &mut &'a [u8]) -> Result<&'a [u8]> {
	let data: &'a [u8] = input;
	let end = data
		.iter()
		.position(|&b| b == b'\r' || b == b'\n')
		.ok_or(ParseError::Incomplete(Needed::Unknown))?;

	let (line, mut rest) = data.split_at(end);
	crlf(&mut rest)?;

	*input = rest;
	Ok(line)
}

/// Consume a signed decimal integer terminated by CRLF.
#[inline]
fn int_line(input: &mut &[u8]) -> Result<i64> {
	let neg = match input.split_first() {
		Some((&b'-', rest)) => {
			*input = rest;
			true
		}
		Some((&b'+', rest)) => {
			*input = rest;
			false
		}
		Some(_) => false,
		None => return Err(ParseError::Incomplete(Needed::Size(1))),
	};

	if input.is_empty() {
		return Err(ParseError::Incomplete(Needed::Size(1)));
	}

	let magnitude = parse_digits(input)?;
	let n = if neg {
		0i64.checked_sub_unsigned(magnitude)
	} else {
		i64::try_from(magnitude).ok()
	}
	.ok_or(ParseError::Integer)?;

	if input.is_empty() {
		return Err(ParseError::Incomplete(Needed::Size(1)));
	}

	crlf(input)?;
	Ok(n)
}

/// Parse a non-empty run of ASCII digits as an unsigned integer, stopping at the first non-digit.
#[inline]
pub fn parse_digits(input: &mut &[u8]) -> Result<u64> {
	let mut rest = *input;
	let mut n: u64 = 0;
	while let Some((&b, next)) = rest.split_first() {
		if !b.is_ascii_digit() {
			break;
		}

		n = n
			.checked_mul(10)
			.and_then(|n| n.checked_add((b - b'0') as u64))
			.ok_or(ParseError::Integer)?;
		rest = next;
	}

	if rest.len() == input.len() {
		return Err(ParseError::Integer);
	}

	*input = rest;
	Ok(n)
}

/// Parse a RESP string.
#[inline]
pub fn parse_str<'a>(input: &mut &'a [u8]) -> Result<&'a str> {
	let mut rest = *input;
	tag(&mut rest, b'+')?;
	let str = from_utf8(line(&mut rest)?)?;

	*input = rest;
	Ok(str)
}

/// Parse a RESP error.
#[inline]
pub fn parse_err<'a>(input: &mut &'a [u8]) -> Result<&'a str> {
	let mut rest = *input;
	tag(&mut rest, b'-')?;
	let str = from_utf8(line(&mut rest)?)?;

	*input = rest;
	Ok(str)
}

/// Parse a RESP integer.
#[inline]
pub fn parse_int(input: &mut &[u8]) -> Result<i64> {
	let mut rest = *input;
	tag(&mut rest, b':')?;
	let int = int_line(&mut rest)?;

	*input = rest;
	Ok(int)
}

/// Parse a RESP bulk string.
#[inline]
pub fn parse_bytes<'a>(input: &mut &'a [u8]) -> Result<Option<&'a [u8]>> {
	let mut rest = *input;
	tag(&mut rest, b'$')?;

	let len = match int_line(&mut rest)? {
		-1 => {
			*input = rest;
			return Ok(None);
		}
		len => usize::try_from(len).map_err(|_| ParseError::Length(len))?,
	};

	let needed = len.checked_add(2).ok_or(ParseError::Length(len as i64))?;
	if rest.len() < needed {
		return Err(ParseError::Incomplete(Needed::Size(needed - rest.len())));
	}

	let (bytes, mut rest) = rest.split_at(len);
	crlf(&mut rest)?;

	*input = rest;
	Ok(Some(bytes))
}

/// Parse the length of a RESP array. Parsing the array elements is handled by the other parsers.
#[inline]
pub fn parse_array(input: &mut &[u8]) -> Result<i64> {
	let mut rest = *input;
	tag(&mut rest, b'*')?;
	let len = int_line(&mut rest)?;

	*input = rest;
	Ok(len)
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_parse_str() {
		let mut resp = "+OK\r\n".as_bytes();
		let res = parse_str(&mut resp).expect("Parsed string");

		assert_eq!(0, resp.len());
		assert_eq!("OK", res);
	}

	#[test]
	fn test_parse_int() {
		let mut resp = ":10\r\n".as_bytes();
		let res = parse_int(&mut resp).expect("Parsed int");

		assert_eq!(0, resp.len());
		assert_eq!(10, res);
	}

	#[test]
	fn test_parse_bytes() {
		let mut resp = "$6\r\nfoobar\r\n".as_bytes();
		let res = parse_bytes(&mut resp).expect("Parsed bytes");

		assert_eq!(0, resp.len());
		assert_eq!(Some("foobar".as_bytes()), res);
	}

	#[test]
	fn test_parse_empty_bytes() {
		let mut resp = "$0\r\n\r\n".as_bytes();
		let res = parse_bytes(&mut resp).expect("Parsed bytes");

		assert_eq!(0, resp.len());
		assert_eq!(Some([].as_slice()), res);
	}

	#[test]
	fn test_parse_null_bytes() {
		let mut resp = "$-1\r\n".as_bytes();
		let res = parse_bytes(&mut resp).expect("Parsed bytes");

		assert_eq!(0, resp.len());
		assert_eq!(None, res);
	}

	#[test]
	fn test_parse_incomplete_bytes() {
		let mut resp = "$6\r\nfoo".as_bytes();
		let err = parse_bytes(&mut resp).expect_err("Incomplete bytes");

		assert_eq!(ParseError::Incomplete(Needed::Size(5)), err);
		assert_eq!(b"$6\r\nfoo", resp);
	}

	#[test]
	fn test_parse_array() {
		let mut resp = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_bytes();
		let res = parse_array(&mut resp).expect("Parsed bytes");

		assert_eq!(18, resp.len());
		assert_eq!(2, res);
	}

	#[test]
	fn test_parse_empty_array() {
		let mut resp = "*0\r\n".as_bytes();
		let res = parse_array(&mut resp).expect("Parsed bytes");

		assert_eq!(0, resp.len());
		assert_eq!(0, res);
	}

	#[test]
	fn test_parse_null_array() {
		let mut resp = "*-1\r\n".as_bytes();
		let res = parse_array(&mut resp).expect("Parsed bytes");

		assert_eq!(0, resp.len());
		assert_eq!(-1, res);
	}

	#[test]
	fn test_parse_digits() {
		let mut input = "123-456".as_bytes();
		assert_eq!(Ok(123), parse_digits(&mut input));
		assert_eq!(b"-456", input);

		let mut input = "18446744073709551615".as_bytes();
		assert_eq!(Ok(u64::MAX), parse_digits(&mut input));
		assert_eq!(0, input.len());

		for data in ["", "-1", "abc", "18446744073709551616"] {
			let mut input = data.as_bytes();
			assert_eq!(
				Err(ParseError::Integer),
				parse_digits(&mut input),
				"{data:?}"
			);
			assert_eq!(data.as_bytes(), input);
		}
	}

	/// Summarize the result of `parse` for comparison against [`EDGE_CASES`], checking that the
	/// input is only advanced on success.
	fn class<'a, T: std::fmt::Debug>(
		parse: impl FnOnce(&mut &'a [u8]) -> Result<T>,
		input: &'a [u8],
	) -> String {
		let mut rest = input;
		match parse(&mut rest) {
			Ok(v) => format!("Ok({:?}, {:?})", String::from_utf8_lossy(rest), v),
			Err(e) => {
				assert_eq!(rest, input, "input advanced on error");
				match e {
					ParseError::Incomplete(_) => "Incomplete".into(),
					_ => "Error".into(),
				}
			}
		}
	}

	/// Expected results for each input, in order: `parse_str`, `parse_int`, `parse_bytes`,
	/// `parse_array`.
	#[rustfmt::skip]
	const EDGE_CASES: &[(&[u8], [&str; 4])] = &[
		(b"", ["Incomplete", "Incomplete", "Incomplete", "Incomplete"]),
		(b"+", ["Incomplete", "Error", "Error", "Error"]),
		(b"+OK", ["Incomplete", "Error", "Error", "Error"]),
		(b"+OK\r", ["Incomplete", "Error", "Error", "Error"]),
		(b"+OK\r\n", ["Ok(\"\", \"OK\")", "Error", "Error", "Error"]),
		(b"+OK\r\nrest", ["Ok(\"rest\", \"OK\")", "Error", "Error", "Error"]),
		(b"+O\rK\r\n", ["Error", "Error", "Error", "Error"]),
		(b"+OK\n", ["Error", "Error", "Error", "Error"]),
		(b"+\r\n", ["Ok(\"\", \"\")", "Error", "Error", "Error"]),
		(b":", ["Error", "Incomplete", "Error", "Error"]),
		(b":-", ["Error", "Incomplete", "Error", "Error"]),
		(b":10", ["Error", "Incomplete", "Error", "Error"]),
		(b":10\r\n", ["Error", "Ok(\"\", 10)", "Error", "Error"]),
		(b":-10\r\n", ["Error", "Ok(\"\", -10)", "Error", "Error"]),
		(b":+10\r\n", ["Error", "Ok(\"\", 10)", "Error", "Error"]),
		(b":1a\r\n", ["Error", "Error", "Error", "Error"]),
		(b":99999999999999999999\r\n", ["Error", "Error", "Error", "Error"]),
		(b":-9223372036854775808\r\n", ["Error", "Ok(\"\", -9223372036854775808)", "Error", "Error"]),
		(b":-9223372036854775809\r\n", ["Error", "Error", "Error", "Error"]),
		(b":9223372036854775807\r\n", ["Error", "Ok(\"\", 9223372036854775807)", "Error", "Error"]),
		(b":9223372036854775808\r\n", ["Error", "Error", "Error", "Error"]),
		(b":-\r\n", ["Error", "Error", "Error", "Error"]),
		(b"$", ["Error", "Error", "Incomplete", "Error"]),
		(b"$3", ["Error", "Error", "Incomplete", "Error"]),
		(b"$3\r\n", ["Error", "Error", "Incomplete", "Error"]),
		(b"$3\r\nfo", ["Error", "Error", "Incomplete", "Error"]),
		(b"$3\r\nfoo", ["Error", "Error", "Incomplete", "Error"]),
		(b"$3\r\nfoo\r", ["Error", "Error", "Incomplete", "Error"]),
		(b"$3\r\nfoo\r\n", ["Error", "Error", "Ok(\"\", Some([102, 111, 111]))", "Error"]),
		(b"$3\r\nfoobar", ["Error", "Error", "Error", "Error"]),
		(b"$0\r\n\r\n", ["Error", "Error", "Ok(\"\", Some([]))", "Error"]),
		(b"$-1\r\n", ["Error", "Error", "Ok(\"\", None)", "Error"]),
		(b"$-2\r\n", ["Error", "Error", "Error", "Error"]),
		(b"$2\r\n12\r\n", ["Error", "Error", "Ok(\"\", Some([49, 50]))", "Error"]),
		(b"$2\r\n\xff\xfe\r\n", ["Error", "Error", "Ok(\"\", Some([255, 254]))", "Error"]),
		(b"*", ["Error", "Error", "Error", "Incomplete"]),
		(b"*2\r\n", ["Error", "Error", "Error", "Ok(\"\", 2)"]),
		(b"*-1\r\n", ["Error", "Error", "Error", "Ok(\"\", -1)"]),
		(b"*0\r\n", ["Error", "Error", "Error", "Ok(\"\", 0)"]),
		(b"-ERR\r\n", ["Error", "Error", "Error", "Error"]),
		(b"x", ["Error", "Error", "Error", "Error"]),
	];

	#[test]
	fn test_edge_cases() {
		for (input, expected) in EDGE_CASES {
			let actual = [
				class(parse_str, input),
				class(parse_int, input),
				class(parse_bytes, input),
				class(parse_array, input),
			];

			assert_eq!(actual, *expected, "{:?}", String::from_utf8_lossy(input));
		}
	}
}
