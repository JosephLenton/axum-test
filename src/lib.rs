//!
//! Axum Test is a library for writing tests for web servers written using Axum:
//!
//!  * You create a [`TestServer`] within a test,
//!  * use that to build [`TestRequest`] against your application,
//!  * receive back a [`TestResponse`],
//!  * then assert the response is how you expect.
//!
//! It includes built in support for serializing and deserializing request and response bodies using Serde,
//! support for cookies and headers, and other common bits you would expect.
//!
//! `TestServer` will pass http requests directly to the handler,
//! or can be run on a random IP / Port address.
//!
//! ## Getting Started
//!
//! Create a [`TestServer`] running your Axum [`Router`](::axum::Router):
//!
//! ```rust
//! # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
//! #
//! use ::axum::Router;
//! use ::axum::extract::Json;
//! use ::axum::routing::put;
//! use ::axum_test::TestServer;
//! use ::serde_json::json;
//! use ::serde_json::Value;
//!
//! async fn route_put_user(Json(user): Json<Value>) -> () {
//!     // todo
//! }
//!
//! let my_app = Router::new()
//!     .route("/users", put(route_put_user));
//!
//! let server = TestServer::new(my_app)?;
//! #
//! # Ok(())
//! # }
//! ```
//!
//! Then make requests against it:
//!
//! ```rust
//! # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
//! #
//! # use ::axum::Router;
//! # use ::axum::extract::Json;
//! # use ::axum::routing::put;
//! # use ::axum_test::TestServer;
//! # use ::serde_json::json;
//! # use ::serde_json::Value;
//! #
//! # async fn put_user(Json(user): Json<Value>) -> () {}
//! #
//! # let my_app = Router::new()
//! #     .route("/users", put(put_user));
//! #
//! # let server = TestServer::new(my_app)?;
//! #
//! let response = server.put("/users")
//!     .json(&json!({
//!         "username": "Terrance Pencilworth",
//!     }))
//!     .await;
//! #
//! # Ok(())
//! # }
//! ```
//!
//! ## Crate Features
//!
//! Here are a list of all features so far that can be enabled:
//!
//!  * `all` _off by default_, turns on all features below.
//!  * `pretty-assertions` **on by default**, uses the [pretty assertions crate](https://crates.io/crates/pretty_assertions) for the output to the `assert_*` functions.
//!  * `yaml` _off by default_, adds support for sending, receiving, and asserting, [yaml content](https://yaml.org/).
//!  * `msgpack` _off by default_, adds support for sending, receiving, and asserting, [msgpack content](https://msgpack.org/index.html).
//!  * `axum-extra` _off by default_, adds support for the `TypedPath` from [axum-extra](https://crates.io/crates/axum-extra).
//!
//! ## Features
//!
//! ### Auto Cookie Saving 🍪
//!
//! This feature allows the server to save cookies and reuse these on future requests.
//! For example saving session cookies, like a browser would.
//!
//! This feature is disabled by default, and can be enabled by setting `save_cookies` to true on the [`TestServerConfig`],
//! and passing this to the [`TestServer`] on construction.
//!
//! ```rust
//! # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
//! #
//! use ::axum::Router;
//! use ::axum_test::TestServer;
//! use ::axum_test::TestServerConfig;
//!
//! let my_app = Router::new();
//! let config = TestServerConfig::builder()
//!     .save_cookies()
//!     .build();
//!
//! let server = TestServer::new_with_config(my_app, config)?;
//! #
//! # Ok(())
//! # }
//! ```
//!
//! When you make a request, any cookies returned will be reused by the next request,
//! created by that same server.
//!
//! You can turn this on or off per request, using `TestRequest::do_save_cookies`
//! and `TestRequest::do_not_save_cookies`.
//!
//! ### Content Type 📇
//!
//! When performing a request, it will start with no content type at all.
//!
//! You can set a default type for all `TestRequest` objects to use,
//! by setting the `default_content_type` in the `TestServerConfig`.
//! When creating the `TestServer` instance, using `new_with_config`.
//!
//! ```rust
//! # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
//! #
//! use ::axum::Router;
//! use ::axum_test::TestServer;
//! use ::axum_test::TestServerConfig;
//!
//! let my_app = Router::new();
//! let config = TestServerConfig::builder()
//!     .default_content_type("application/json")
//!     .build();
//!
//! let server = TestServer::new_with_config(my_app, config)?;
//! #
//! # Ok(())
//! # }
//! ```
//!
//! If there is no default, then a `TestRequest` will try to guess the content type.
//! Such as setting `application/json` when calling `TestRequest::json`,
//! and `text/plain` when calling `TestRequest::text`.
//! This will never override any default content type provided.
//!
//! Finally on each `TestRequest`, one can set the content type to use.
//! By calling `TestRequest::content_type` on it.
//!
//! ```rust
//! # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
//! #
//! use ::axum::Router;
//! use ::axum::extract::Json;
//! use ::axum::routing::put;
//! use ::axum_test::TestServer;
//! use ::serde_json::json;
//! use ::serde_json::Value;
//!
//! async fn put_user(Json(user): Json<Value>) -> () {
//!     // todo
//! }
//!
//! let my_app = Router::new()
//!     .route("/users", put(put_user));
//!
//! let server = TestServer::new(my_app)?;
//!
//! let response = server.put("/users")
//!     .content_type(&"application/json")
//!     .json(&json!({
//!         "username": "Terrance Pencilworth",
//!     }))
//!     .await;
//! #
//! # Ok(())
//! # }
//! ```
//!
//! ### Fail Fast ⚡️
//!
//! This library includes a mode to have requests panic if they are outside of the 2xx range,
//! unless marked by calling [`TestRequest::expect_failure()`](crate::TestRequest::expect_failure()).
//! This is intentional to aid with writing tests, and to help catch errors quickly when making code changes.
//!
//! This behaviour is off by default, and can be enabled by setting [`TestServerConfig::expect_success_by_default`](crate::TestServerConfig::expect_success_by_default) to true
//! when creating a new `TestServer`.
//!

#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub(crate) mod internals;

pub mod multipart;

pub mod transport_layer;
pub mod util;

mod test_request;
pub use self::test_request::*;

mod test_response;
pub use self::test_response::*;

mod test_server_config_builder;
pub use self::test_server_config_builder::*;

mod test_server_config;
pub use self::test_server_config::*;

mod test_server;
pub use self::test_server::*;

#[cfg(feature = "ws")]
mod test_web_socket;
#[cfg(feature = "ws")]
pub use self::test_web_socket::*;
#[cfg(feature = "ws")]
pub use ::tokio_tungstenite::tungstenite::Message as WsMessage;

mod transport;
pub use self::transport::*;

pub use ::http;

#[cfg(test)]
mod integrated_test_cookie_saving {
    use super::*;

    use ::axum::extract::Request;
    use ::axum::routing::get;
    use ::axum::routing::post;
    use ::axum::routing::put;
    use ::axum::Router;
    use ::axum_extra::extract::cookie::Cookie as AxumCookie;
    use ::axum_extra::extract::cookie::CookieJar;
    use ::cookie::time::OffsetDateTime;
    use ::cookie::Cookie;
    use ::http_body_util::BodyExt;
    use ::std::time::Duration;

    const TEST_COOKIE_NAME: &'static str = &"test-cookie";

    async fn get_cookie(cookies: CookieJar) -> (CookieJar, String) {
        let cookie = cookies.get(&TEST_COOKIE_NAME);
        let cookie_value = cookie
            .map(|c| c.value().to_string())
            .unwrap_or_else(|| "cookie-not-found".to_string());

        (cookies, cookie_value)
    }

    async fn put_cookie(mut cookies: CookieJar, request: Request) -> (CookieJar, &'static str) {
        let body_bytes = request
            .into_body()
            .collect()
            .await
            .expect("Should extract the body")
            .to_bytes();
        let body_text: String = String::from_utf8_lossy(&body_bytes).to_string();
        let cookie = AxumCookie::new(TEST_COOKIE_NAME, body_text);
        cookies = cookies.add(cookie);

        (cookies, &"done")
    }

    async fn post_expire_cookie(mut cookies: CookieJar) -> (CookieJar, &'static str) {
        let mut cookie = AxumCookie::new(TEST_COOKIE_NAME, "expired".to_string());
        let expired_time = OffsetDateTime::now_utc() - Duration::from_secs(1);
        cookie.set_expires(expired_time);
        cookies = cookies.add(cookie);

        (cookies, &"done")
    }

    fn new_test_router() -> Router {
        Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie))
            .route("/expire", post(post_expire_cookie))
    }

    #[tokio::test]
    async fn it_should_not_pass_cookies_created_back_up_to_server_by_default() {
        // Run the server.
        let server = TestServer::new(new_test_router()).expect("Should create test server");

        // Create a cookie.
        server.put(&"/cookie").text(&"new-cookie").await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "cookie-not-found");
    }

    #[tokio::test]
    async fn it_should_not_pass_cookies_created_back_up_to_server_when_turned_off() {
        // Run the server.
        let server = TestServerConfig::builder()
            .do_not_save_cookies()
            .build_server(new_test_router())
            .expect("Should create test server");

        // Create a cookie.
        server.put(&"/cookie").text(&"new-cookie").await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "cookie-not-found");
    }

    #[tokio::test]
    async fn it_should_pass_cookies_created_back_up_to_server_automatically() {
        // Run the server.
        let server = TestServerConfig::builder()
            .save_cookies()
            .build_server(new_test_router())
            .expect("Should create test server");

        // Create a cookie.
        server.put(&"/cookie").text(&"cookie-found!").await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "cookie-found!");
    }

    #[tokio::test]
    async fn it_should_pass_cookies_created_back_up_to_server_when_turned_on_for_request() {
        // Run the server.
        let server = TestServerConfig::builder()
            .do_not_save_cookies() // it's off by default!
            .build_server(new_test_router())
            .expect("Should create test server");

        // Create a cookie.
        server
            .put(&"/cookie")
            .text(&"cookie-found!")
            .do_save_cookies()
            .await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "cookie-found!");
    }

    #[tokio::test]
    async fn it_should_wipe_cookies_cleared_by_request() {
        // Run the server.
        let server = TestServerConfig::builder()
            .do_not_save_cookies() // it's off by default!
            .build_server(new_test_router())
            .expect("Should create test server");

        // Create a cookie.
        server
            .put(&"/cookie")
            .text(&"cookie-found!")
            .do_save_cookies()
            .await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").clear_cookies().await.text();

        assert_eq!(response_text, "cookie-not-found");
    }

    #[tokio::test]
    async fn it_should_wipe_cookies_cleared_by_test_server() {
        // Run the server.
        let mut server = TestServerConfig::builder()
            .do_not_save_cookies() // it's off by default!
            .build_server(new_test_router())
            .expect("Should create test server");

        // Create a cookie.
        server
            .put(&"/cookie")
            .text(&"cookie-found!")
            .do_save_cookies()
            .await;

        server.clear_cookies();

        // Check it comes back.
        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "cookie-not-found");
    }

    #[tokio::test]
    async fn it_should_send_cookies_added_to_request() {
        // Run the server.
        let server = TestServerConfig::builder()
            .do_not_save_cookies() // it's off by default!
            .build_server(new_test_router())
            .expect("Should create test server");

        // Check it comes back.
        let cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");

        let response_text = server.get(&"/cookie").add_cookie(cookie).await.text();

        assert_eq!(response_text, "my-custom-cookie");
    }

    #[tokio::test]
    async fn it_should_send_cookies_added_to_test_server() {
        // Run the server.
        let mut server = TestServerConfig::builder()
            .do_not_save_cookies() // it's off by default!
            .build_server(new_test_router())
            .expect("Should create test server");

        // Check it comes back.
        let cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");
        server.add_cookie(cookie);

        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "my-custom-cookie");
    }

    #[tokio::test]
    async fn it_should_remove_expired_cookies_from_later_requests() {
        // Run the server.
        let mut server = TestServer::new(new_test_router()).expect("Should create test server");
        server.do_save_cookies();

        // Create a cookie.
        server.put(&"/cookie").text(&"cookie-found!").await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").await.text();
        assert_eq!(response_text, "cookie-found!");

        server.post(&"/expire").await;

        // Then expire the cookie.
        let found_cookie = server.post(&"/expire").await.maybe_cookie(TEST_COOKIE_NAME);
        assert!(found_cookie.is_some());

        // It's no longer found
        let response_text = server.get(&"/cookie").await.text();
        assert_eq!(response_text, "cookie-not-found");
    }
}

#[cfg(feature = "typed-routing")]
#[cfg(test)]
mod integrated_test_typed_routing_and_query {
    use super::*;

    use ::axum::extract::Query;
    use ::axum::Router;
    use ::axum_extra::routing::RouterExt;
    use ::axum_extra::routing::TypedPath;
    use ::serde::Deserialize;
    use ::serde::Serialize;

    #[derive(TypedPath, Deserialize)]
    #[typed_path("/path-query/:id")]
    struct TestingPathQuery {
        id: u32,
    }

    #[derive(Serialize, Deserialize)]
    struct QueryParams {
        param: String,
        other: Option<String>,
    }

    async fn route_get_with_param(
        TestingPathQuery { id }: TestingPathQuery,
        Query(params): Query<QueryParams>,
    ) -> String {
        let query = params.param;
        if let Some(other) = params.other {
            format!("get {id}, {query}&{other}")
        } else {
            format!("get {id}, {query}")
        }
    }

    fn new_app() -> Router {
        Router::new().typed_get(route_get_with_param)
    }

    #[tokio::test]
    async fn it_should_send_typed_get_with_query_params() {
        let server = TestServer::new(new_app()).unwrap();
        let path = TestingPathQuery { id: 123 }.with_query_params(QueryParams {
            param: "with-typed-query".to_string(),
            other: None,
        });

        server
            .typed_get(&path)
            .expect_success()
            .await
            .assert_text("get 123, with-typed-query");
    }

    #[tokio::test]
    async fn it_should_send_typed_get_with_added_query_param() {
        let server = TestServer::new(new_app()).unwrap();
        let path = TestingPathQuery { id: 123 };

        server
            .typed_get(&path)
            .add_query_param("param", "with-added-query")
            .expect_success()
            .await
            .assert_text("get 123, with-added-query");
    }

    #[tokio::test]
    async fn it_should_send_both_typed_and_added_query() {
        let server = TestServer::new(new_app()).unwrap();
        let path = TestingPathQuery { id: 123 }.with_query_params(QueryParams {
            param: "with-typed-query".to_string(),
            other: None,
        });

        server
            .typed_get(&path)
            .add_query_param("other", "with-added-query")
            .expect_success()
            .await
            .assert_text("get 123, with-typed-query&with-added-query");
    }

    #[tokio::test]
    async fn it_should_send_replaced_query_when_cleared() {
        let server = TestServer::new(new_app()).unwrap();
        let path = TestingPathQuery { id: 123 }.with_query_params(QueryParams {
            param: "with-typed-query".to_string(),
            other: Some("with-typed-other".to_string()),
        });

        server
            .typed_get(&path)
            .clear_query_params()
            .add_query_param("param", "with-added-query")
            .expect_success()
            .await
            .assert_text("get 123, with-added-query");
    }
}
