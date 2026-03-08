use crate::TestResponse;
use crate::internals::body_fmt::BodyFmtKind;
use bytes::Bytes;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

#[derive(Debug)]
pub struct BodyFmt<'a> {
    kind: BodyFmtKind,
    bytes: &'a Bytes,
}

impl<'a> BodyFmt<'a> {
    pub fn from_test_response(response: &'a TestResponse) -> Self {
        let kind = BodyFmtKind::from_maybe_content_type(response.maybe_content_type());
        let bytes = response.as_bytes();

        Self { kind, bytes }
    }
}

impl Display for BodyFmt<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.kind.fmt_body(f, self.bytes)
    }
}
