use anyhow::anyhow;
use anyhow::Result;
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::extract::Request as AxumRequest;
use axum::response::Response as AxumResponse;
use axum::serve::IncomingStream;
use std::convert::Infallible;
use tower::Service;
use url::Url;

use crate::internals::HttpTransportLayer;
use crate::transport_layer::IntoTransportLayer;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;
use crate::util::spawn_serve;

impl<S, C> IntoTransportLayer for IntoMakeServiceWithConnectInfo<S, C>
where
    for<'a> C: axum::extract::connect_info::Connected<IncomingStream<'a>>,
    S: Service<AxumRequest, Response = AxumResponse, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send,
{
    fn into_http_transport_layer(
        self,
        builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        let (socket_addr, tcp_listener, maybe_reserved_port) =
            builder.tcp_listener_with_reserved_port()?;

        let serve_handle = spawn_serve(tcp_listener, self);
        let server_address = format!("http://{socket_addr}");
        let server_url: Url = server_address.parse()?;

        Ok(Box::new(HttpTransportLayer::new(
            serve_handle,
            maybe_reserved_port,
            server_url,
        )))
    }

    fn into_mock_transport_layer(self) -> Result<Box<dyn TransportLayer>> {
        Err(anyhow!("`IntoMakeServiceWithConnectInfo` cannot be mocked, as it's underlying implementation requires a real connection. Set the `TestServerConfig` to run with a transport of `HttpRandomPort`, or a `HttpIpPort`."))
    }

    fn into_default_transport(
        self,
        builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        self.into_http_transport_layer(builder)
    }
}

#[cfg(test)]
mod test_into_http_transport_layer_for_into_make_service_with_connect_info {
    use crate::TestServer;
    use axum::extract::Request;
    use axum::routing::get;
    use axum::Router;
    use axum::ServiceExt;
    use std::net::SocketAddr;
    use tower::Layer;
    use tower_http::normalize_path::NormalizePathLayer;

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
        let server = TestServer::builder()
            .http_transport()
            .build(app)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }

    #[tokio::test]
    async fn it_should_create_and_run_with_router_wrapped_service() {
        // Build an application with a route.
        let router = Router::new().route("/ping", get(get_ping));
        let normalized_router = NormalizePathLayer::trim_trailing_slash().layer(router);
        let app = ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(
            normalized_router,
        );

        // Run the server.
        let server = TestServer::builder()
            .http_transport()
            .build(app)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }
}

#[cfg(test)]
mod test_into_mock_transport_layer_for_into_make_service_with_connect_info {
    use axum::routing::get;
    use axum::Router;
    use std::net::SocketAddr;
    use crate::TestServer;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    async fn it_should_panic_when_creating_test_using_mock() {
        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service_with_connect_info::<SocketAddr>();

        // Build the server.
        let result = TestServer::builder().mock_transport().build(app);
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);

        assert_eq!(err_msg, "`IntoMakeServiceWithConnectInfo` cannot be mocked, as it's underlying implementation requires a real connection. Set the `TestServerConfig` to run with a transport of `HttpRandomPort`, or a `HttpIpPort`.");
    }
}
