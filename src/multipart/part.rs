use anyhow::Context;
use bytes::Bytes;
use http::HeaderName;
use http::HeaderValue;
use mime::Mime;
use std::fmt::Debug;
use std::fmt::Display;

///
/// For creating a section of a MultipartForm.
///
/// Use [`Part::text()`](crate::multipart::Part::text()) and [`Part::bytes()`](crate::multipart::Part::bytes()) for creating new instances.
/// Then attach them to a `MultipartForm` using [`MultipartForm::add_part()`](crate::multipart::MultipartForm::add_part()).
///
#[derive(Debug, Clone)]
pub struct Part {
    pub(crate) bytes: Bytes,
    pub(crate) file_name: Option<String>,
    pub(crate) mime_type: Mime,
    pub(crate) headers: Vec<(HeaderName, HeaderValue)>,
}

impl Part {
    /// Creates a new part of a multipart form, that will send text.
    ///
    /// The default mime type for this part will be `text/plain`,
    pub fn text<T>(text: T) -> Self
    where
        T: Display,
    {
        let bytes = text.to_string().into_bytes().into();

        Self::new(bytes, mime::TEXT_PLAIN)
    }

    /// Creates a new part of a multipart form, that will upload bytes.
    ///
    /// The default mime type for this part will be `application/octet-stream`,
    pub fn bytes<B>(bytes: B) -> Self
    where
        B: Into<Bytes>,
    {
        Self::new(bytes.into(), mime::APPLICATION_OCTET_STREAM)
    }

    fn new(bytes: Bytes, mime_type: Mime) -> Self {
        Self {
            bytes,
            file_name: None,
            mime_type,
            headers: Default::default(),
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

    /// Adds a header to be sent with the Part of this Multiform.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum_test::TestServer;
    /// use axum_test::multipart::MultipartForm;
    /// use axum_test::multipart::Part;
    ///
    /// let app = Router::new();
    /// let server = TestServer::new(app)?;
    ///
    /// let readme_bytes = include_bytes!("../../README.md");
    /// let readme_part = Part::bytes(readme_bytes.as_slice())
    ///     .file_name(&"README.md")
    ///     // Add a header to the Part
    ///     .add_header("x-text-category", "readme");
    ///
    /// let multipart_form = MultipartForm::new()
    ///     .add_part("file", readme_part);
    ///
    /// let response = server.post(&"/my-form")
    ///     .multipart(multipart_form)
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    pub fn add_header<N, V>(mut self, name: N, value: V) -> Self
    where
        N: TryInto<HeaderName>,
        N::Error: Debug,
        V: TryInto<HeaderValue>,
        V::Error: Debug,
    {
        let header_name: HeaderName = name
            .try_into()
            .expect("Failed to convert header name to HeaderName");
        let header_value: HeaderValue = value
            .try_into()
            .expect("Failed to convert header vlue to HeaderValue");

        self.headers.push((header_name, header_value));
        self
    }
}

#[cfg(test)]
mod test_text {
    use super::*;

    #[test]
    fn it_should_contain_text_given() {
        let part = Part::text("some_text");

        let output = String::from_utf8_lossy(&part.bytes);
        assert_eq!(output, "some_text");
    }

    #[test]
    fn it_should_use_mime_type_text() {
        let part = Part::text("some_text");
        assert_eq!(part.mime_type, mime::TEXT_PLAIN);
    }
}

#[cfg(test)]
mod test_byes {
    use super::*;

    #[test]
    fn it_should_contain_bytes_given() {
        let bytes = "some_text".as_bytes();
        let part = Part::bytes(bytes);

        let output = String::from_utf8_lossy(&part.bytes);
        assert_eq!(output, "some_text");
    }

    #[test]
    fn it_should_use_mime_type_octet_stream() {
        let bytes = "some_text".as_bytes();
        let part = Part::bytes(bytes);

        assert_eq!(part.mime_type, mime::APPLICATION_OCTET_STREAM);
    }
}

#[cfg(test)]
mod test_file_name {
    use super::*;

    #[test]
    fn it_should_use_file_name_given() {
        let mut part = Part::text("some_text");

        assert_eq!(part.file_name, None);
        part = part.file_name("my-text.txt");
        assert_eq!(part.file_name, Some("my-text.txt".to_string()));
    }
}

#[cfg(test)]
mod test_mime_type {
    use super::*;

    #[test]
    fn it_should_use_mime_type_set() {
        let mut part = Part::text("some_text");

        assert_eq!(part.mime_type, mime::TEXT_PLAIN);
        part = part.mime_type("application/json");
        assert_eq!(part.mime_type, mime::APPLICATION_JSON);
    }

    #[test]
    #[should_panic]
    fn it_should_error_if_invalid_mime_type() {
        let part = Part::text("some_text");
        part.mime_type("🦊");

        assert!(false);
    }
}
