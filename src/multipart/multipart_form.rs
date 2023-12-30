use ::axum::body::Body as AxumBody;
use ::rust_multipart_rfc7578_2::client::multipart::Body as CommonMultipartBody;
use ::rust_multipart_rfc7578_2::client::multipart::Form;
use ::std::fmt::Display;
use ::std::io::Cursor;

use crate::multipart::Part;

pub struct MultipartForm {
    inner: Form<'static>,
}

impl MultipartForm {
    pub fn new() -> Self {
        Self {
            inner: Form::default(),
        }
    }

    /// Adds a new key / value pair to be sent.
    pub fn add_text<N, T>(mut self, name: N, text: T) -> Self
    where
        N: Display,
        T: ToString,
    {
        self.inner.add_text(name, text.to_string());
        self
    }

    /// Adds a new key / value pair to be sent.
    pub fn add_part<N>(mut self, name: N, part: Part) -> Self
    where
        N: Display,
    {
        let reader = Cursor::new(part.bytes);
        self.inner
            .add_reader_2(name, reader, part.file_name, part.mime_type);

        self
    }

    pub fn content_type(&self) -> String {
        self.inner.content_type()
    }
}

impl From<MultipartForm> for AxumBody {
    fn from(multipart: MultipartForm) -> Self {
        let inner_body: CommonMultipartBody = multipart.inner.into();
        AxumBody::from_stream(inner_body)
    }
}
