//!
//! Axum Test is a library for writing tests for web servers written using Axum.
//!
//!  * You can spin up a `TestServer` within a test.
//!  * Create requests that will run against that.
//!  * Retrieve what they happen to return.
//!  * Assert that the response works how you expect.
//!
//! It icludes built in suppot with Serde, Cookies,
//! and other common crates for working with the web.
//!
//! ## Getting Started
//!
//! In essence; create your Axum application, create a `TestServer`,
//! and then make requests against it.
//!
//! ```rust
//! # ::tokio_test::block_on(async {
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
//!     .route("/users", put(put_user))
//!     .into_make_service();
//!
//! let server = TestServer::new(my_app)
//!     .unwrap();
//!
//! let response = server.put("/users")
//!     .json(&json!({
//!         "username": "Terrance Pencilworth",
//!     }))
//!     .await;
//! # })
//! ```
//!
//! ## Features
//!
//! ### Auto Cookie Saving 🍪
//!
//! When you build a `TestServer`, you can turn on a feature to automatically save cookies
//! across requests. This is used for automatically saving things like session cookies.
//!
//! ```rust
//! # ::tokio_test::block_on(async {
//! use ::axum::Router;
//! use ::axum_test::TestServer;
//! use ::axum_test::TestServerConfig;
//!
//! let my_app = Router::new()
//!     .into_make_service();
//!
//! let config = TestServerConfig {
//!     save_cookies: true,
//!     ..TestServerConfig::default()
//! };
//! let server = TestServer::new_with_config(my_app, config)
//!     .unwrap();
//! # })
//! ```
//!
//! Then when you make a request, any cookies that are returned will be reused
//! by the next request. This is on a per server basis (it doesn't save across servers).
//!
//! You can turn this on or off per request, using `TestRequest::do_save_cookies'
//! and TestRequest::do_not_save_cookies'.
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
//! # ::tokio_test::block_on(async {
//! use ::axum::Router;
//! use ::axum_test::TestServer;
//! use ::axum_test::TestServerConfig;
//!
//! let my_app = Router::new()
//!     .into_make_service();
//!
//! let config = TestServerConfig {
//!     default_content_type: Some("application/json".to_string()),
//!     ..TestServerConfig::default()
//! };
//!
//! let server = TestServer::new_with_config(my_app, config)
//!     .unwrap();
//! # })
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
//! # ::tokio_test::block_on(async {
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
//!     .route("/users", put(put_user))
//!     .into_make_service();
//!
//! let server = TestServer::new(my_app)
//!     .unwrap();
//!
//! let response = server.put("/users")
//!     .content_type(&"application/json")
//!     .json(&json!({
//!         "username": "Terrance Pencilworth",
//!     }))
//!     .await;
//! # })
//! ```
//!
//! ### Fail Fast
//!
//! This library is written to panic quickly. For example by default a response will presume to
//! succeed and will panic if they don't (which you can change).
//! Functions to retreive cookies and headers will by default panic if they aren't found.
//!
//! This behaviour is unorthodox for Rust, however it is intentional to aid with writing tests.
//! Where you want the test to fail as quickly, and skip on writing error handling code.
//!

pub(crate) mod internals;

mod into_test_server_core;
pub use self::into_test_server_core::*;

mod test_server;
pub use self::test_server::*;

mod test_server_config;
pub use self::test_server_config::*;

mod test_request;
pub use self::test_request::*;

mod test_response;
pub use self::test_response::*;

pub mod util;

pub use ::hyper::http;

#[cfg(test)]
mod test_get {
    use super::*;

    use ::axum::routing::get;
    use ::axum::Router;

    use crate::util::new_random_socket_addr;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    async fn it_should_get() {
        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }

    #[tokio::test]
    async fn it_should_get_using_absolute_path() {
        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();

        // Run the server.
        let address = new_random_socket_addr().unwrap();
        let ip = address.ip();
        let port = address.port();
        let test_config = TestServerConfig {
            ip: Some(ip),
            port: Some(port),
            ..TestServerConfig::default()
        };
        let server =
            TestServer::new_with_config(app, test_config).expect("Should create test server");

        // Get the request.
        let absolute_url = format!("http://{ip}:{port}/ping");
        server.get(&absolute_url).await.assert_text(&"pong!");
    }

    #[tokio::test]
    async fn it_should_not_get_using_absolute_path_if_restricted() {
        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();

        // Run the server.
        let address = new_random_socket_addr().unwrap();
        let ip = address.ip();
        let port = address.port();
        let test_config = TestServerConfig {
            ip: Some(ip),
            port: Some(port),
            restrict_requests_with_http_schema: true, // Key part of the test!
            ..TestServerConfig::default()
        };
        let server =
            TestServer::new_with_config(app, test_config).expect("Should create test server");

        // Get the request.
        let absolute_url = format!("http://{ip}:{port}/ping");
        server
            .get(&absolute_url)
            .expect_failure()
            .await
            .assert_status_not_found();
    }
}

#[cfg(test)]
mod test_content_type {
    use super::*;

    use ::axum::http::header::CONTENT_TYPE;
    use ::axum::http::HeaderMap;
    use ::axum::routing::get;
    use ::axum::Router;

    async fn get_content_type(headers: HeaderMap) -> String {
        headers
            .get(CONTENT_TYPE)
            .map(|h| h.to_str().unwrap().to_string())
            .unwrap_or_else(|| "".to_string())
    }

    #[tokio::test]
    async fn it_should_not_set_a_content_type_by_default() {
        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", get(get_content_type))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server.get(&"/content_type").await.text();

        assert_eq!(text, "");
    }

    #[tokio::test]
    async fn it_should_default_to_server_content_type_when_present() {
        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", get(get_content_type))
            .into_make_service();

        // Run the server.
        let config = TestServerConfig {
            default_content_type: Some("text/plain".to_string()),
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        // Get the request.
        let text = server.get(&"/content_type").await.text();

        assert_eq!(text, "text/plain");
    }

    #[tokio::test]
    async fn it_should_override_server_content_type_when_present() {
        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", get(get_content_type))
            .into_make_service();

        // Run the server.
        let config = TestServerConfig {
            default_content_type: Some("text/plain".to_string()),
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        // Get the request.
        let text = server
            .get(&"/content_type")
            .content_type(&"application/json")
            .await
            .text();

        assert_eq!(text, "application/json");
    }

    #[tokio::test]
    async fn it_should_set_content_type_when_present() {
        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", get(get_content_type))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server
            .get(&"/content_type")
            .content_type(&"application/json")
            .await
            .text();

        assert_eq!(text, "application/json");
    }
}

#[cfg(test)]
mod test_cookies {
    use super::*;

    use ::axum::extract::RawBody;
    use ::axum::routing::get;
    use ::axum::routing::put;
    use ::axum::Router;
    use ::axum_extra::extract::cookie::Cookie as AxumCookie;
    use ::axum_extra::extract::cookie::CookieJar;
    use ::cookie::Cookie;
    use ::hyper::body::to_bytes;

    const TEST_COOKIE_NAME: &'static str = &"test-cookie";

    async fn get_cookie(cookies: CookieJar) -> (CookieJar, String) {
        let cookie = cookies.get(&TEST_COOKIE_NAME);
        let cookie_value = cookie
            .map(|c| c.value().to_string())
            .unwrap_or_else(|| "cookie-not-found".to_string());

        (cookies, cookie_value)
    }

    async fn put_cookie(
        mut cookies: CookieJar,
        RawBody(body): RawBody,
    ) -> (CookieJar, &'static str) {
        let body_bytes = to_bytes(body)
            .await
            .expect("Should turn the body into bytes");
        let body_text: String = String::from_utf8_lossy(&body_bytes).to_string();
        let cookie = AxumCookie::new(TEST_COOKIE_NAME, body_text);
        cookies = cookies.add(cookie);

        (cookies, &"done")
    }

    #[tokio::test]
    async fn it_should_not_pass_cookies_created_back_up_to_server_by_default() {
        // Build an application with a route.
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Create a cookie.
        server.put(&"/cookie").text(&"new-cookie").await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "cookie-not-found");
    }

    #[tokio::test]
    async fn it_should_not_pass_cookies_created_back_up_to_server_when_turned_off() {
        // Build an application with a route.
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie))
            .into_make_service();

        // Run the server.
        let server = TestServer::new_with_config(
            app,
            TestServerConfig {
                save_cookies: false,
                ..TestServerConfig::default()
            },
        )
        .expect("Should create test server");

        // Create a cookie.
        server.put(&"/cookie").text(&"new-cookie").await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "cookie-not-found");
    }

    #[tokio::test]
    async fn it_should_pass_cookies_created_back_up_to_server_automatically() {
        // Build an application with a route.
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie))
            .into_make_service();

        // Run the server.
        let server = TestServer::new_with_config(
            app,
            TestServerConfig {
                save_cookies: true,
                ..TestServerConfig::default()
            },
        )
        .expect("Should create test server");

        // Create a cookie.
        server.put(&"/cookie").text(&"cookie-found!").await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "cookie-found!");
    }

    #[tokio::test]
    async fn it_should_pass_cookies_created_back_up_to_server_when_turned_on_for_request() {
        // Build an application with a route.
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie))
            .into_make_service();

        // Run the server.
        let server = TestServer::new_with_config(
            app,
            TestServerConfig {
                save_cookies: false, // it's off by default!
                ..TestServerConfig::default()
            },
        )
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
        // Build an application with a route.
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie))
            .into_make_service();

        // Run the server.
        let server = TestServer::new_with_config(
            app,
            TestServerConfig {
                save_cookies: false, // it's off by default!
                ..TestServerConfig::default()
            },
        )
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
        // Build an application with a route.
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new_with_config(
            app,
            TestServerConfig {
                save_cookies: false, // it's off by default!
                ..TestServerConfig::default()
            },
        )
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
        // Build an application with a route.
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie))
            .into_make_service();

        // Run the server.
        let server = TestServer::new_with_config(
            app,
            TestServerConfig {
                save_cookies: false, // it's off by default!
                ..TestServerConfig::default()
            },
        )
        .expect("Should create test server");

        // Check it comes back.
        let cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");

        let response_text = server.get(&"/cookie").add_cookie(cookie).await.text();

        assert_eq!(response_text, "my-custom-cookie");
    }

    #[tokio::test]
    async fn it_should_send_cookies_added_to_test_server() {
        // Build an application with a route.
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new_with_config(
            app,
            TestServerConfig {
                save_cookies: false, // it's off by default!
                ..TestServerConfig::default()
            },
        )
        .expect("Should create test server");

        // Check it comes back.
        let cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");
        server.add_cookie(cookie);

        let response_text = server.get(&"/cookie").await.text();

        assert_eq!(response_text, "my-custom-cookie");
    }
}
