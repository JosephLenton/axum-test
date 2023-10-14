use ::axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::hyper::server::conn::AddrStream;

use crate::internals::MockTransportLayer;
use crate::internals::TransportLayer;

/// This exists to gloss over the differences between Axum's
/// [`IntoMakeService`](::axum::routing::IntoMakeService) and [`IntoMakeServiceWithConnectInfo`](::axum::extract::connect_info::IntoMakeServiceWithConnectInfo) types.
///
/// This is a trait for turning those types into a thread,
/// that is running a web server.
///
pub trait IntoMockTransportLayer {
    fn into_mock_transport_layer(self) -> Box<dyn TransportLayer>;
}

impl IntoMockTransportLayer for Router<()> {
    fn into_mock_transport_layer(self) -> Box<dyn TransportLayer> {
        self.into_make_service().into_mock_transport_layer()
    }
}

impl IntoMockTransportLayer for IntoMakeService<Router> {
    fn into_mock_transport_layer(self) -> Box<dyn TransportLayer> {
        let transport_layer = MockTransportLayer::new(self);
        Box::new(transport_layer)
    }
}

impl<C> IntoMockTransportLayer for IntoMakeServiceWithConnectInfo<Router, C>
where
    for<'a> C: axum::extract::connect_info::Connected<&'a AddrStream>,
{
    fn into_mock_transport_layer(self) -> Box<dyn TransportLayer> {
        unimplemented!("todo")
    }
}

/*
#[cfg(test)]
mod test_into_test_server_thread_for_into_make_service {
    use ::axum::extract::State;
    use ::axum::routing::get;
    use ::axum::Router;

    use crate::TestServer;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    async fn get_state(State(count): State<u32>) -> String {
        format!("count is {}", count)
    }

    #[tokio::test]
    async fn it_should_create_and_test_with_make_into_service() {
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
    async fn it_should_create_and_test_with_make_into_service_with_state() {
        // Build an application with a route.
        let app = Router::new()
            .route("/count", get(get_state))
            .with_state(123)
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}

#[cfg(test)]
mod test_into_test_server_thread_for_into_make_service_with_connect_info {
    use ::axum::routing::get;
    use ::axum::Router;
    use ::std::net::SocketAddr;

    use crate::TestServer;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    async fn it_should_create_and_test_with_make_into_service_with_connect_info() {
        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service_with_connect_info::<SocketAddr>();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }
}
 */
