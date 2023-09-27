use ::axum::Router;
use ::auto_future::AutoFuture;
use ::bytes::Bytes;
use ::http::header;
use ::http::HeaderName;
use ::http::HeaderValue;
use ::http::Method;
use ::http::request::Builder;
use ::hyper::body::Body;
use ::serde::Serialize;
use ::serde_json::to_vec as json_to_vec;
use ::serde_json::Value;
use ::serde_urlencoded::to_string;
use ::std::fmt::Debug;
use ::std::fmt::Display;
use ::std::future::IntoFuture;
use ::tower::util::ServiceExt;
use super::test_response::TestClientResponse;

const JSON_CONTENT_TYPE: &'static str = &"application/json";
const FORM_CONTENT_TYPE: &'static str = &"application/x-www-form-urlencoded";
const TEXT_CONTENT_TYPE: &'static str = &"text/plain";

///
/// A `TestClient` is for building and executing a HTTP request to the [`TestClient`](crate::TestClient).
///
/// ## Building
///
/// Requests are created by the [`TestClient`](crate::TestClient), using it's builder functions.
/// They correspond to the appropriate HTTP method.
/// Such as [`TestClient::get()`](crate::TestClient::get()), [`TestClient::post()`](crate::TestClient::post()), and so on.
///
/// See that for documentation.
///
/// ## Customizing
///
/// The `TestClient` allows the caller to fill in the rest of the request
/// to be sent to the server. Including the headers, the body, cookies,
/// and the content type, using the relevant functions.
///
/// The TestClient struct provides a number of methods to set up the request,
/// such as json, text, bytes, expect_failure, content_type, etc.
///
/// ## Sending
///
/// Once fully configured you send the request by awaiting the request object.
///
#[derive(Debug)]
pub struct TestClient {
	body: Body,
	method: Method,
	uri: String,
	headers: Vec<(HeaderName, HeaderValue)>,
	routes: Router,
	content_type: Option<String>,
}

impl TestClient {
	pub fn new(routes: Router) -> Self {
		Self {
			body: Body::empty(),
			method: Method::GET,
			uri: "/".to_string(),
			headers: vec![],
			routes,
			content_type: None,
		}
	}

	/// Set the body of the request to send up as Json,
    /// and changes the content type to `application/json`.
	pub async fn json2(self, body: &Value) -> TestClientResponse {
		let header = HeaderValue::from_static("application/json");
		let mut request = self.add_header(header::CONTENT_TYPE, header);
		request.body = Body::from(serde_json::to_vec(body).unwrap());
		request.send().await
	}

	pub fn json<J>(self, body: &J) -> Self
    where
        J: ?Sized + Serialize,
    {
        let body_bytes = json_to_vec(body).expect("It should serialize the content into JSON");

        self.bytes(body_bytes.into())
            .content_type(JSON_CONTENT_TYPE)
    }

	/// Sets the body of the request, with the content type
    /// of 'application/x-www-form-urlencoded'.
    pub fn form<F>(self, body: &F) -> Self
    where
        F: ?Sized + Serialize,
    {
        let body_text = to_string(body).expect("It should serialize the content into a Form");

        self.bytes(body_text.into()).content_type(FORM_CONTENT_TYPE)
    }

    /// Set raw text as the body of the request,
    /// and sets the content type to `text/plain`.
    pub fn text<T>(self, raw_text: T) -> Self
    where
        T: Display,
    {
        let body_text = format!("{}", raw_text);

        self.bytes(body_text.into()).content_type(TEXT_CONTENT_TYPE)
    }

	/// Set raw bytes as the body of the request.
    ///
    /// The content type is left unchanged.
    pub fn bytes(mut self, body_bytes: Bytes) -> Self {
        let body: Body = body_bytes.into();

        self.body = body;
        self
    }

	/// Set the content type to use for this request in the header.
    pub fn content_type(mut self, content_type: &str) -> Self {
        self.content_type = Some(content_type.to_string());
        self
    }

	/// Creates a HTTP GET request to the path.
	pub async fn get(self, path: &str) -> TestClientResponse {
		let mut build = self.method(Method::GET).set_uri(path);
		build.body = Body::empty();
		build.send().await
	}

	/// Creates a HTTP POST request to the given path.
	pub fn post(self, path: &str) -> Self {
		let build = self.method(Method::POST);
		build.set_uri(path)
	}

	/// Creates a HTTP PATCH request to the path.
    /*pub fn patch(&self, path: &str) -> Self {
        let build = self.method(Method::PATCH);
		build.set_uri(path)
    }*/

	/// Creates a HTTP PUT request to the path.
	pub fn put(self, path: &str) -> Self {
		let build = self.method(Method::PUT);
		build.set_uri(path)
	}

	/// Creates a HTTP DELETE request to the path.
	pub fn delete(self, path: &str) -> Self {
		let build = self.method(Method::DELETE);
		build.set_uri(path)
	}

	fn method(mut self, method: Method) -> Self {
		self.method = method;
		self
	}

	fn set_uri(mut self, uri: &str) -> Self {
		self.uri = uri.to_string();
		self
	}

	pub fn add_header(mut self, name: impl Into<HeaderName>, value: impl Into<HeaderValue>) -> Self {
		self.headers.push((name.into(), value.into()));
		self
	}

	async fn send(self) -> TestClientResponse {
		let mut builder = Builder::new();

		// Iterate through headers
		for (name, value) in self.headers {
			builder = builder.header(name, value);
		}

		let user_requested_path = self.uri.clone();

		// Add the method, uri and body
		let request = builder.method(self.method).uri(self.uri).body(self.body).unwrap();
		TestClientResponse::new(self.routes.oneshot(request).await.unwrap(), user_requested_path).await
	}
}

impl IntoFuture for TestClient {
    type Output = TestClientResponse;
    type IntoFuture = AutoFuture<TestClientResponse>;

    fn into_future(self) -> Self::IntoFuture {
        AutoFuture::new(async { self.send().await })
    }
}