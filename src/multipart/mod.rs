//!
//! This supplies the building blocks for sending multipart forms using
//! [`TestRequest::multipart()`](crate::TestRequest::multipart()).
//!
//! The request body can be built using [`MultipartForm`] and [`Part`].
//!
//! # Simple example
//!
//! ```rust
//! # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
//! #
//! use ::axum::Router;
//! use ::axum_test::TestServer;
//! use ::axum_test::multipart::MultipartForm;
//!
//! let app = Router::new();
//! let server = TestServer::new(app)?;
//!
//! let multipart_form = MultipartForm::new()
//!     .add_text("name", "Joe")
//!     .add_text("animals", "foxes");
//!
//! let response = server.post(&"/my-form")
//!     .multipart(multipart_form)
//!     .await;
//! #
//! # Ok(()) }
//! ```
//!
//! # Sending byte parts
//!
//! ```rust
//! # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
//! #
//! use ::axum::Router;
//! use ::axum_test::TestServer;
//! use ::axum_test::multipart::MultipartForm;
//! use ::axum_test::multipart::Part;
//!
//! let app = Router::new();
//! let server = TestServer::new(app)?;
//!
//! let image_bytes = include_bytes!("../../README.md");
//! let image_part = Part::bytes(image_bytes.as_slice())
//!     .file_name(&"README.md")
//!     .mime_type(&"text/markdown");
//!
//! let multipart_form = MultipartForm::new()
//!     .add_part("file", image_part);
//!
//! let response = server.post(&"/my-form")
//!     .multipart(multipart_form)
//!     .await;
//! #
//! # Ok(()) }
//! ```
//!

mod multipart_form;
pub use self::multipart_form::*;

mod part;
pub use self::part::*;
