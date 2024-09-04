use anyhow::Context;
use bytes::Bytes;
use mime::Mime;
use std::fmt::Display;

///
/// For creating a section of a MultipartForm.
///
/// Use [`Part::text()`](crate::multipart::Part::text()) and [`Part::bytes()`](crate::multipart::Part::bytes()) for creating new instances.
/// Then attach them to a `MultipartForm` using [`MultipartForm::add_part()`](crate::multipart::MultipartForm::add_part()).
///
pub struct Part {
    pub(crate) bytes: Bytes,
    pub(crate) file_name: Option<String>,
    pub(crate) mime_type: Mime,
}

impl Part {
    /// Creates a new part of a multipart form, that will send text.
    ///
    /// The default mime type for this part will be `text/plain`,
    pub fn text<T>(text: T) -> Self
    where
        T: Display,
    {
        Self {
            bytes: text.to_string().into_bytes().into(),
            file_name: None,
            mime_type: mime::TEXT_PLAIN,
        }
    }

    /// Creates a new part of a multipart form, that will upload bytes.
    ///
    /// The default mime type for this part will be `application/octet-stream`,
    pub fn bytes<B>(bytes: B) -> Self
    where
        B: Into<Bytes>,
    {
        Self {
            bytes: bytes.into(),
            file_name: None,
            mime_type: mime::APPLICATION_OCTET_STREAM,
        }
    }

    /// Sets the file name for this part of a multipart form.
    ///
    /// By default there is no filename. This will set one.
    pub fn file_name<T>(mut self, file_name: T) -> Self
    where
        T: Display,
    {
        self.file_name = Some(file_name.to_string());
        self
    }

    /// Sets the mime type for this part of a multipart form.
    ///
    /// The default mime type is `text/plain` or `application/octet-stream`,
    /// depending on how this instance was created.
    /// This function will replace that.
    pub fn mime_type<M>(mut self, mime_type: M) -> Self
    where
        M: AsRef<str>,
    {
        let raw_mime_type = mime_type.as_ref();
        let parsed_mime_type = raw_mime_type
            .parse()
            .with_context(|| format!("Failed to parse '{raw_mime_type}' as a Mime type"))
            .unwrap();

        self.mime_type = parsed_mime_type;
        self
    }
}
