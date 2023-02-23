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

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    async fn it_sound_get() {
        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
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
    use ::axum_extra::extract::cookie::Cookie;
    use ::axum_extra::extract::cookie::CookieJar;
    use ::hyper::body::to_bytes;

    async fn get_cookie(cookies: CookieJar) -> (CookieJar, String) {
        let cookie = cookies.get("test-cookie");
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
        let cookie = Cookie::new("test-cookie", body_text);
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
}
