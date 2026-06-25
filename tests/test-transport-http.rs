use axum::Router;
use axum::routing::get;
use reserve_port::ReservedSocketAddr;
use std::net::TcpListener;
use axum_test::TestServer;

#[tokio::test]
async fn it_should_start_a_http_test_server_with_a_tcp_listener() {
    async fn get_ping() -> &'static str {
        "pong!"
    }

    // Build an application with a route.
    let app = Router::new().route("/ping", get(get_ping));

    // Reserve an address
    let reserved_address = ReservedSocketAddr::reserve_random_socket_addr().unwrap();
    let ip = reserved_address.ip();
    let port = reserved_address.port();

    let tcp_listener = TcpListener::bind(&reserved_address).unwrap();

    // Run the server.
    let server = TestServer::builder()
        .http_transport_with_tcp_listener(tcp_listener)
        .expect_success_by_default()
        .build(app);

    // Get the request normally.
    server.get(&"/ping").await.assert_text("pong!");

    // Get the request.
    let absolute_url = format!("http://{ip}:{port}/ping");
    let response = server.get(&absolute_url).await;

    response.assert_text(&"pong!");
    let request_path = response.request_url();
    assert_eq!(request_path.to_string(), format!("http://{ip}:{port}/ping"));
}
