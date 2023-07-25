use ::anyhow::Context;
use ::anyhow::Result;
use ::axum::Server as AxumServer;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::http::Method;
use ::lazy_static::lazy_static;
use ::regex::Regex;
use ::regex::RegexBuilder;
use ::serde::Serialize;
use ::std::net::TcpListener;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::tokio::task::JoinHandle;

use crate::util::new_socket_addr_from_defaults;
use crate::IntoTestServerThread;
use crate::TestRequest;
use crate::TestRequestConfig;
use crate::TestServerConfig;

mod server_shared_state;
pub(crate) use self::server_shared_state::*;

lazy_static! {
    static ref STARTS_HTTP_REGEX: Regex = RegexBuilder::new("^http(s?)://(.+)")
        .case_insensitive(true)
        .build()
        .unwrap();
}

///
/// The `TestServer` runs your application,
/// allowing you to make web requests against it.
///
/// You can make a request against the `TestServer` by calling the
/// [`crate::TestServer::get()`], [`crate::TestServer::post()`], [`crate::TestServer::put()`],
/// [`crate::TestServer::delete()`], and [`crate::TestServer::patch()`] methods.
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use ::axum::Json;
/// use ::axum::routing::Router;
/// use ::axum::routing::get;
/// use ::serde::Deserialize;
/// use ::serde::Serialize;
///
/// use ::axum_test::TestServer;
///
/// let app = Router::new()
///     .route(&"/test", get(|| async { "hello!" }))
///     .into_make_service();
///
/// let server = TestServer::new(app)?;
///
/// // The different responses one can make:
/// let get_response = server.get(&"/todo").await;
/// let post_response = server.post(&"/todo").await;
/// let put_response = server.put(&"/todo").await;
/// let delete_response = server.delete(&"/todo").await;
/// let patch_response = server.patch(&"/todo").await;
/// #
/// # Ok(())
/// # }
/// ```
///
#[derive(Debug)]
pub struct TestServer {
    state: Arc<Mutex<ServerSharedState>>,
    server_thread: JoinHandle<()>,
    server_address: String,
    save_cookies: bool,
    expect_success_by_default: bool,
    default_content_type: Option<String>,
    is_requests_http_restricted: bool,
}

impl TestServer {
    /// This will run the given Axum app,
    /// and run it on a random local address.
    ///
    /// This is the same as creating a new `TestServer` with a configuration,
    /// and passing `TestServerConfig::default()`.
    ///
    pub fn new<A>(app: A) -> Result<Self>
    where
        A: IntoTestServerThread,
    {
        Self::new_with_config(app, TestServerConfig::default())
    }

    /// This very similar to [`TestServer::new()`],
    /// however you can customise some of the configuration.
    /// This includes which port to run on, or default settings.
    ///
    /// See the [`TestServerConfig`] for more information on each configuration setting.
    pub fn new_with_config<A>(app: A, config: TestServerConfig) -> Result<Self>
    where
        A: IntoTestServerThread,
    {
        let socket_address = new_socket_addr_from_defaults(config.ip, config.port)
            .context("Cannot create socket address for use")?;
        let listener = TcpListener::bind(socket_address)
            .with_context(|| "Failed to create TCPListener for TestServer")?;
        let server_builder = AxumServer::from_tcp(listener)
            .with_context(|| "Failed to create ::axum::Server for TestServer")?;

        let server_thread = app.into_server_thread(server_builder);

        let shared_state = ServerSharedState::new();
        let shared_state_mutex = Mutex::new(shared_state);
        let state = Arc::new(shared_state_mutex);

        let this = Self {
            state,
            server_thread,
            server_address: socket_address.to_string(),
            save_cookies: config.save_cookies,
            expect_success_by_default: config.expect_success_by_default,
            default_content_type: config.default_content_type,
            is_requests_http_restricted: config.restrict_requests_with_http_schema,
        };

        Ok(this)
    }

    /// Returns the local web address for the test server.
    ///
    /// By default this will be something like `0.0.0.0:1234`,
    /// where `1234` is a randomly assigned port numbr.
    pub fn server_address<'a>(&'a self) -> &'a str {
        &self.server_address
    }

    /// Clears all of the cookies stored internally.
    pub fn clear_cookies(&mut self) {
        ServerSharedState::clear_cookies(&mut self.state)
            .with_context(|| format!("Trying to clear_cookies"))
            .unwrap()
    }

    /// Adds extra cookies to be used on *all* future requests.
    ///
    /// Any cookies which have the same name as the new cookies,
    /// will get replaced.
    pub fn add_cookies(&mut self, cookies: CookieJar) {
        ServerSharedState::add_cookies(&mut self.state, cookies)
            .with_context(|| format!("Trying to add_cookies"))
            .unwrap()
    }

    /// Adds a cookie to be included on *all* future requests.
    ///
    /// If a cookie with the same name already exists,
    /// then it will be replaced.
    pub fn add_cookie(&mut self, cookie: Cookie) {
        ServerSharedState::add_cookie(&mut self.state, cookie)
            .with_context(|| format!("Trying to add_cookie"))
            .unwrap()
    }

    /// Adds query parameters to be sent with this request.
    pub fn add_query_params<V>(&mut self, query_params: V)
    where
        V: Serialize,
    {
        ServerSharedState::add_query_params(&mut self.state, query_params)
            .with_context(|| format!("Trying to add_query_params"))
            .unwrap()
    }

    /// Adds query parameters to be sent on *all* future requests.
    pub fn add_query_param<V>(&mut self, key: &str, value: V)
    where
        V: Serialize,
    {
        ServerSharedState::add_query_param(&mut self.state, key, value)
            .with_context(|| format!("Trying to add_query_param"))
            .unwrap()
    }

    /// Clears all query params set.
    pub fn clear_query_params(&mut self) {
        ServerSharedState::clear_query_params(&mut self.state)
            .with_context(|| format!("Trying to clear_query_params"))
            .unwrap()
    }

    /// Creates a HTTP GET request to the path.
    pub fn get(&self, path: &str) -> TestRequest {
        self.method(Method::GET, path)
    }

    /// Creates a HTTP POST request to the given path.
    pub fn post(&self, path: &str) -> TestRequest {
        self.method(Method::POST, path)
    }

    /// Creates a HTTP PATCH request to the path.
    pub fn patch(&self, path: &str) -> TestRequest {
        self.method(Method::PATCH, path)
    }

    /// Creates a HTTP PUT request to the path.
    pub fn put(&self, path: &str) -> TestRequest {
        self.method(Method::PUT, path)
    }

    /// Creates a HTTP DELETE request to the path.
    pub fn delete(&self, path: &str) -> TestRequest {
        self.method(Method::DELETE, path)
    }

    /// Creates a HTTP request, to the path given, using the given method.
    pub fn method(&self, method: Method, path: &str) -> TestRequest {
        let debug_method = method.clone();
        let config = self.test_request_config(method, path);
        let maybe_request = TestRequest::new(self.state.clone(), config);

        maybe_request
            .with_context(|| {
                format!(
                    "Trying to create internal request for {} {}",
                    debug_method, path
                )
            })
            .unwrap()
    }

    pub(crate) fn test_request_config(&self, method: Method, path: &str) -> TestRequestConfig {
        let full_request_path =
            build_request_path(&self.server_address, path, self.is_requests_http_restricted);

        TestRequestConfig {
            is_saving_cookies: self.save_cookies,
            is_expecting_success_by_default: self.expect_success_by_default,
            content_type: self.default_content_type.clone(),
            full_request_path,
            method,
            path: path.to_string(),
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.server_thread.abort();
    }
}

fn build_request_path(
    root_path: &str,
    sub_path: &str,
    is_requests_http_restructed: bool,
) -> String {
    if sub_path == "" {
        return format!("http://{}", root_path.to_string());
    }

    if sub_path.starts_with("/") {
        return format!("http://{}{}", root_path, sub_path);
    }

    if !is_requests_http_restructed {
        if starts_with_http(sub_path) {
            return sub_path.to_string();
        }
    }

    format!("http://{}/{}", root_path, sub_path)
}

fn starts_with_http(path: &str) -> bool {
    STARTS_HTTP_REGEX.is_match(path)
}

#[cfg(test)]
mod starts_with_http {
    use super::*;

    #[test]
    fn it_should_be_true_for_http() {
        assert_eq!(starts_with_http(&"http://example.com"), true);
    }

    #[test]
    fn it_should_be_true_for_http_mixed_case() {
        assert_eq!(starts_with_http(&"hTtP://example.com"), true);
    }

    #[test]
    fn it_should_be_false_for_http_on_own() {
        assert_eq!(starts_with_http(&"http://"), false);
    }

    #[test]
    fn it_should_be_false_for_http_in_middle() {
        assert_eq!(starts_with_http(&"something/http://"), false);
    }

    #[test]
    fn it_should_be_true_for_https() {
        assert_eq!(starts_with_http(&"https://example.com"), true);
    }

    #[test]
    fn it_should_be_true_for_https_mixed_case() {
        assert_eq!(starts_with_http(&"hTtPs://example.com"), true);
    }

    #[test]
    fn it_should_be_false_for_https_on_own() {
        assert_eq!(starts_with_http(&"https://"), false);
    }

    #[test]
    fn it_should_be_false_for_https_in_middle() {
        assert_eq!(starts_with_http(&"something/https://"), false);
    }
}

#[cfg(test)]
mod server_address {
    use super::*;
    use ::axum::Router;
    use ::local_ip_address::local_ip;
    use ::regex::Regex;

    #[tokio::test]
    async fn it_should_return_address_used_from_config() {
        let ip = local_ip().unwrap();
        let config = TestServerConfig {
            ip: Some(ip),
            port: Some(3000),
            ..TestServerConfig::default()
        };

        // Build an application with a route.
        let app = Router::new().into_make_service();
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        let expected_ip_port = format!("{}:3000", ip);
        assert_eq!(server.server_address(), expected_ip_port);
    }

    #[tokio::test]
    async fn it_should_return_default_address_without_ending_slash() {
        let app = Router::new().into_make_service();
        let server = TestServer::new(app).expect("Should create test server");

        let address_regex = Regex::new("^127\\.0\\.0\\.1:[0-9]+$").unwrap();
        let is_match = address_regex.is_match(&server.server_address());
        assert!(is_match);
    }
}

#[cfg(test)]
mod test_add_query_params {
    use ::axum::extract::Query;
    use ::axum::routing::get;
    use ::axum::Router;

    use ::serde::Deserialize;
    use ::serde::Serialize;
    use ::serde_json::json;

    use crate::TestServer;

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam {
        message: String,
    }

    async fn get_query_param(Query(params): Query<QueryParam>) -> String {
        params.message
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam2 {
        message: String,
        other: String,
    }

    async fn get_query_param_2(Query(params): Query<QueryParam2>) -> String {
        format!("{}-{}", params.message, params.other)
    }

    #[tokio::test]
    async fn it_should_pass_up_query_params_from_serialization() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_param))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(QueryParam {
            message: "it works".to_string(),
        });

        // Get the request.
        server.get(&"/query").await.assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_query_params_from_pairs() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_param))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(&[("message", "it works")]);

        // Get the request.
        server.get(&"/query").await.assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_params() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query-2", get(get_query_param_2))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(&[("message", "it works"), ("other", "yup")]);

        // Get the request.
        server.get(&"/query-2").await.assert_text(&"it works-yup");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_calls() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query-2", get(get_query_param_2))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(&[("message", "it works")]);
        server.add_query_params(&[("other", "yup")]);

        // Get the request.
        server.get(&"/query-2").await.assert_text(&"it works-yup");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_json() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query-2", get(get_query_param_2))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(json!({
            "message": "it works",
            "other": "yup"
        }));

        // Get the request.
        server.get(&"/query-2").await.assert_text(&"it works-yup");
    }
}

#[cfg(test)]
mod test_add_query_param {
    use ::axum::extract::Query;
    use ::axum::routing::get;
    use ::axum::Router;

    use ::serde::Deserialize;
    use ::serde::Serialize;

    use crate::TestServer;

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam {
        message: String,
    }

    async fn get_query_param(Query(params): Query<QueryParam>) -> String {
        params.message
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam2 {
        message: String,
        other: String,
    }

    async fn get_query_param_2(Query(params): Query<QueryParam2>) -> String {
        format!("{}-{}", params.message, params.other)
    }

    #[tokio::test]
    async fn it_should_pass_up_query_params_from_pairs() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_param))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_param("message", "it works");

        // Get the request.
        server.get(&"/query").await.assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_calls() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query-2", get(get_query_param_2))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_param("message", "it works");
        server.add_query_param("other", "yup");

        // Get the request.
        server.get(&"/query-2").await.assert_text(&"it works-yup");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_calls_across_server_and_request() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query-2", get(get_query_param_2))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_param("message", "it works");

        // Get the request.
        server
            .get(&"/query-2")
            .add_query_param("other", "yup")
            .await
            .assert_text(&"it works-yup");
    }
}

#[cfg(test)]
mod test_clear_query_params {
    use ::axum::extract::Query;
    use ::axum::routing::get;
    use ::axum::Router;

    use ::serde::Deserialize;
    use ::serde::Serialize;

    use crate::TestServer;

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParams {
        first: Option<String>,
        second: Option<String>,
    }

    async fn get_query_params(Query(params): Query<QueryParams>) -> String {
        format!(
            "has first? {}, has second? {}",
            params.first.is_some(),
            params.second.is_some()
        )
    }

    #[tokio::test]
    async fn it_should_clear_all_params_set() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_params))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(QueryParams {
            first: Some("first".to_string()),
            second: Some("second".to_string()),
        });
        server.clear_query_params();

        // Get the request.
        server
            .get(&"/query")
            .await
            .assert_text(&"has first? false, has second? false");
    }

    #[tokio::test]
    async fn it_should_clear_all_params_set_and_allow_replacement() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_params))
            .into_make_service();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(QueryParams {
            first: Some("first".to_string()),
            second: Some("second".to_string()),
        });
        server.clear_query_params();
        server.add_query_params(QueryParams {
            first: Some("first".to_string()),
            second: Some("second".to_string()),
        });

        // Get the request.
        server
            .get(&"/query")
            .await
            .assert_text(&"has first? true, has second? true");
    }
}

#[cfg(test)]
mod test_expect_success_by_default {
    use super::*;

    use ::axum::routing::get;
    use ::axum::Router;

    #[tokio::test]
    async fn it_should_not_panic_by_default_if_accessing_404_route() {
        let app = Router::new().into_make_service();
        let server = TestServer::new(app).expect("Should create test server");

        server.get(&"/some_unknown_route").await;
    }

    #[tokio::test]
    async fn it_should_not_panic_by_default_if_accessing_200_route() {
        let app = Router::new()
            .route("/known_route", get(|| async { "🦊🦊🦊" }))
            .into_make_service();
        let server = TestServer::new(app).expect("Should create test server");

        server.get(&"/known_route").await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_by_default_if_accessing_404_route_and_expect_success_on() {
        let app = Router::new().into_make_service();
        let server = TestServer::new_with_config(
            app,
            TestServerConfig {
                expect_success_by_default: true,
                ..TestServerConfig::default()
            },
        )
        .expect("Should create test server");

        server.get(&"/some_unknown_route").await;
    }

    #[tokio::test]
    async fn it_should_not_panic_by_default_if_accessing_200_route_and_expect_success_on() {
        let app = Router::new()
            .route("/known_route", get(|| async { "🦊🦊🦊" }))
            .into_make_service();
        let server = TestServer::new_with_config(
            app,
            TestServerConfig {
                expect_success_by_default: true,
                ..TestServerConfig::default()
            },
        )
        .expect("Should create test server");

        server.get(&"/known_route").await;
    }
}
