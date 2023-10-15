use ::axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::hyper::server::conn::AddrIncoming;
use ::hyper::server::conn::AddrStream;
use ::hyper::server::Builder;
use ::tokio::spawn;

use crate::internals::HttpTransportLayer;
use crate::transport_layer::TransportLayer;

/// This exists to gloss over the differences between Axum's
/// [`IntoMakeService`](::axum::routing::IntoMakeService) and [`IntoMakeServiceWithConnectInfo`](::axum::extract::connect_info::IntoMakeServiceWithConnectInfo) types.
///
/// This is a trait for turning those types into a thread,
/// that is running a web server.
///
pub trait IntoHttpTransportLayer {
    fn into_http_transport_layer(
        self,
        server_builder: Builder<AddrIncoming>,
    ) -> Box<dyn TransportLayer>;
}

impl IntoHttpTransportLayer for Router<()> {
    fn into_http_transport_layer(
        self,
        server_builder: Builder<AddrIncoming>,
    ) -> Box<dyn TransportLayer> {
        self.into_make_service()
            .into_http_transport_layer(server_builder)
    }
}

impl IntoHttpTransportLayer for IntoMakeService<Router> {
    fn into_http_transport_layer(
        self,
        server_builder: Builder<AddrIncoming>,
    ) -> Box<dyn TransportLayer> {
        let server = server_builder.serve(self);
        let server_handle = spawn(async move {
            server.await.expect("Expect server to start serving");
        });

        Box::new(HttpTransportLayer::new(server_handle))
    }
}

impl<C> IntoHttpTransportLayer for IntoMakeServiceWithConnectInfo<Router, C>
where
    for<'a> C: axum::extract::connect_info::Connected<&'a AddrStream>,
{
    fn into_http_transport_layer(
        self,
        server_builder: Builder<AddrIncoming>,
    ) -> Box<dyn TransportLayer> {
        let server = server_builder.serve(self);
        let server_handle = spawn(async move {
            server.await.expect("Expect server to start serving");
        });

        Box::new(HttpTransportLayer::new(server_handle))
    }
}

#[cfg(test)]
mod test_into_http_transport_layer_for_router {
    use ::axum::extract::State;
    use ::axum::routing::get;
    use ::axum::Router;

    use crate::TestServer;
    use crate::TestServerConfig;
    use crate::Transport;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    async fn get_state(State(count): State<u32>) -> String {
        format!("count is {}", count)
    }

    #[tokio::test]
    async fn it_should_create_and_test_with_make_into_service() {
        // Build an application with a route.
        let app: Router = Router::new().route("/ping", get(get_ping));

        // Run the server.
        let config = TestServerConfig {
            transport: Transport::HttpRandomPort,
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }

    #[tokio::test]
    async fn it_should_create_and_test_with_make_into_service_with_state() {
        // Build an application with a route.
        let app: Router = Router::new()
            .route("/count", get(get_state))
            .with_state(123);

        // Run the server.
        let config = TestServerConfig {
            transport: Transport::HttpRandomPort,
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}

#[cfg(test)]
mod test_into_http_transport_layer_for_into_make_service {
    use ::axum::extract::State;
    use ::axum::routing::get;
    use ::axum::routing::IntoMakeService;
    use ::axum::Router;

    use crate::TestServer;
    use crate::TestServerConfig;
    use crate::Transport;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    async fn get_state(State(count): State<u32>) -> String {
        format!("count is {}", count)
    }

    #[tokio::test]
    async fn it_should_create_and_test_with_make_into_service() {
        // Build an application with a route.
        let app: IntoMakeService<Router> = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();

        // Run the server.
        let config = TestServerConfig {
            transport: Transport::HttpRandomPort,
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }

    #[tokio::test]
    async fn it_should_create_and_test_with_make_into_service_with_state() {
        // Build an application with a route.
        let app: IntoMakeService<Router> = Router::new()
            .route("/count", get(get_state))
            .with_state(123)
            .into_make_service();

        // Run the server.
        let config = TestServerConfig {
            transport: Transport::HttpRandomPort,
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}

#[cfg(test)]
mod test_into_http_transport_layer_for_into_make_service_with_connect_info {
    use ::axum::routing::get;
    use ::axum::Router;
    use ::std::net::SocketAddr;

    use crate::TestServer;
    use crate::TestServerConfig;
    use crate::Transport;

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
        let config = TestServerConfig {
            transport: Transport::HttpRandomPort,
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }
}
