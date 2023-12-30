use ::anyhow::Context;
use ::bytes::Bytes;
use ::mime::Mime;
use ::std::fmt::Display;

pub struct Part {
    pub(crate) bytes: Bytes,
    pub(crate) file_name: Option<String>,
    pub(crate) mime_type: Option<Mime>,
}

impl Part {
    pub fn text<T>(text: T) -> Self
    where
        T: Display,
    {
        Self {
            bytes: text.to_string().into_bytes().into(),
            file_name: None,
            mime_type: Some(mime::TEXT_PLAIN),
        }
    }

    pub fn bytes<B>(bytes: B) -> Self
    where
        B: Into<Bytes>,
    {
        Self {
            bytes: bytes.into(),
            file_name: None,
            mime_type: None,
        }
    }

    /// Sets the mime type for this multiform part.
    pub fn file_name<T>(mut self, file_name: T) -> Self
    where
        T: Display,
    {
        self.file_name = Some(file_name.to_string());
        self
    }

    /// Sets the mime type for this multiform part.
    pub fn mime_type<M>(mut self, mime_type: M) -> Self
    where
        M: AsRef<str>,
    {
        let raw_mime_type = mime_type.as_ref();
        let parsed_mime_type = raw_mime_type
            .parse()
            .with_context(|| format!("Failed to parse '{raw_mime_type}' as a Mime type"))
            .unwrap();

        self.mime_type = Some(parsed_mime_type);
        self
    }
}
