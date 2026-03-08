use bytes::Bytes;
use bytesize::ByteSize;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

#[derive(Debug, Copy, Clone)]
pub struct BodyBytesFmt<'a>(pub &'a Bytes);

impl<'a> Display for BodyBytesFmt<'a> {
    #[cfg(not(feature = "yaml"))]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let len = self.0.len();
        write!(f, "<Bytes, with len {}>", ByteSize(len as u64))
    }
}
