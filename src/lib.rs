mod test_server;
pub use self::test_server::*;

mod random_socket_address;
pub use self::random_socket_address::*;

#[cfg(test)]
mod test {
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
        let server = TestServer::new_with_random_address(app).expect("Test server to startup");

        // Get the request.
        let response = server.get("/ping").await.unwrap();

        assert_eq!(response.contents, "pong!");
    }
}
