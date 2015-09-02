use std::collections::HashMap;
use std::io::{self, BufRead};
use super::error::MogResult;

/// An extension of the standard library's `BufRead` trait which
/// supports multibyte delimiters.
pub trait BufReadMb: BufRead {
    fn read_until_mb(&mut self, delim: &[u8], buf: &mut Vec<u8>) -> io::Result<usize> {
        use std::io::ErrorKind;

        let mut read = 0;
        loop {
            let (done, used) = {
                let available = match self.fill_buf() {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e)
                };
                match available.windows(delim.len()).position(|x| x == delim) {
                    Some(i) => {
                        buf.extend(&available[..i + delim.len()]);
                        (true, i + delim.len())
                    }
                    None => {
                        buf.extend(available);
                        (false, available.len())
                    }
                }
            };
            self.consume(used);
            read += used;
            if done || used == 0 {
                return Ok(read);
            }
        }
    }
}

impl<B: BufRead> BufReadMb for B {}

/// A trait abstracting the ability to construct something from a byte
/// string.
pub trait FromBytes {
    fn from_bytes(bytes: &[u8]) -> MogResult<Self>;
}

impl<B: FromBytes> FromBytes for Box<B> {
    fn from_bytes(bytes: &[u8]) -> MogResult<Self> {
        B::from_bytes(bytes).map(|b| Box::new(b))
    }
}

/// A trait abstracting the ability to convert something in to a
/// tuple-vec or string hash of arguments, obviously discarding any
/// type-safety.
pub trait ToArgs {
    fn to_args(&self) -> Vec<(String, String)>;

    fn to_args_hash(&self) -> HashMap<String, String> {
        let mut rv = HashMap::new();
        for (k, v) in self.to_args().into_iter() {
            rv.entry(k).or_insert(v);
        }
        rv
    }
}

impl<T: ToArgs + ?Sized> ToArgs for Box<T> {
    fn to_args(&self) -> Vec<(String, String)> {
        use std::ops::Deref;
        self.deref().to_args()
    }
}

impl<'a, T: ToArgs + ?Sized> ToArgs for &'a T {
    fn to_args(&self) -> Vec<(String, String)> {
        use std::ops::Deref;
        self.deref().to_args()
    }
}

#[cfg(test)]
mod tests {
    use super::BufReadMb;
    use std::io::{BufReader, Cursor, Read};

    static CRLF: &'static [u8] = b"\r\n";

    #[test]
    fn read_until_mb() {
        let data = b"This line will end with two bytes:\r\nAnd then a second line.";
        let mut reader = BufReader::new(Cursor::new(data.as_ref()));
        let mut dst_buf = Vec::new();

        reader.read_until_mb(CRLF, &mut dst_buf).unwrap();
        assert_eq!(b"This line will end with two bytes:\r\n".as_ref(), &*dst_buf);

        dst_buf.clear();

        reader.read_until_mb(CRLF, &mut dst_buf).unwrap();
        assert_eq!(b"And then a second line.", &*dst_buf);
    }

    #[test]
    fn read_until_mb_skips_partial_eols() {
        let data = b"This line will end\r with two bytes:\r\nAnd then a second\n line.";
        let mut reader = BufReader::new(Cursor::new(data.as_ref()));
        let mut dst_buf = Vec::new();

        reader.read_until_mb(CRLF, &mut dst_buf).unwrap();
        assert_eq!(b"This line will end\r with two bytes:\r\n".as_ref(), &*dst_buf);

        dst_buf.clear();

        reader.read_until_mb(CRLF, &mut dst_buf).unwrap();
        assert_eq!(b"And then a second\n line.", &*dst_buf);
    }

    #[test]
    fn read_until_mb_ends_with_delimiter() {
        let data = b"This line will end with two bytes:\r\nAnd then a second line, which also ends with two bytes.\r\n";
        let mut reader = BufReader::new(Cursor::new(data.as_ref()));
        let mut dst_buf = Vec::new();

        reader.read_until_mb(CRLF, &mut dst_buf).unwrap();
        assert_eq!(b"This line will end with two bytes:\r\n".as_ref(), &*dst_buf);

        dst_buf.clear();

        reader.read_until_mb(CRLF, &mut dst_buf).unwrap();
        assert_eq!(b"And then a second line, which also ends with two bytes.\r\n".as_ref(), &*dst_buf);

        dst_buf.clear();

        reader.read_to_end(&mut dst_buf).unwrap();
        assert!(dst_buf.is_empty());
    }
}