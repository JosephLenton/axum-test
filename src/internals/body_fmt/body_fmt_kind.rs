use crate::internals::body_fmt::BodyBytesFmt;
use crate::internals::body_fmt::BodyJsonFmt;
use crate::internals::body_fmt::BodyMsgpackFmt;
use crate::internals::body_fmt::BodyTextFmt;
use crate::internals::body_fmt::BodyYamlFmt;
use bytes::Bytes;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

#[derive(Default, Debug, Copy, Clone)]
pub enum BodyFmtKind {
    Text,
    Json,
    Bytes,
    MsgPack,
    Yaml,

    #[default]
    Unknown,
}

impl BodyFmtKind {
    pub fn from_maybe_content_type<S>(maybe_content_type: Option<S>) -> Self
    where
        S: AsRef<str>,
    {
        maybe_content_type
            .as_ref()
            .map(Self::from_content_type)
            .unwrap_or_default()
    }

    pub fn from_content_type<S>(content_type: S) -> Self
    where
        S: AsRef<str>,
    {
        match content_type.as_ref() {
            "application/json" | "text/json" => Self::Json,
            "application/yaml" | "application/x-yaml" | "text/yaml" => Self::Yaml,
            "application/msgpack" => Self::MsgPack,
            "application/octet-stream" => Self::Bytes,

            // Text Content
            s if s.starts_with("text/") => Self::Text,

            // Unknown content type
            _ => Self::Unknown,
        }
    }

    pub fn fmt_body(self, f: &mut Formatter<'_>, bytes: &Bytes) -> FmtResult {
        match self {
            Self::Text => BodyTextFmt(bytes).fmt(f),
            Self::Json => BodyJsonFmt(bytes).fmt(f),
            Self::Bytes => BodyBytesFmt(bytes).fmt(f),
            Self::MsgPack => BodyMsgpackFmt(bytes).fmt(f),
            Self::Yaml => BodyYamlFmt(bytes).fmt(f),
            Self::Unknown => BodyTextFmt(bytes).fmt(f),
        }
    }
}
