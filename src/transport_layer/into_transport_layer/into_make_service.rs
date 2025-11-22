use anyhow::Result;
use axum::extract::Request as AxumRequest;
use axum::response::Response as AxumResponse;
use axum::routing::IntoMakeService;
use std::convert::Infallible;
use tower::Service;
use url::Url;

use crate::internals::HttpTransportLayer;
use crate::internals::MockTransportLayer;
use crate::transport_layer::IntoTransportLayer;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;
use crate::util::spawn_serve;

impl<S> IntoTransportLayer for IntoMakeService<S>
where
    S: Service<AxumRequest, Response = AxumResponse, Error = Infallible>
        + Clone
        + Send
        + Sync
        + 'static,
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
        let transport_layer = MockTransportLayer::new(self);
        Ok(Box::new(transport_layer))
    }
}

#[cfg(test)]
mod test_into_http_transport_layer_for_into_make_service {
    use crate::TestServer;
    use axum::Router;
    use axum::ServiceExt;
    use axum::extract::Request;
    use axum::extract::State;
    use axum::routing::get;
    use tower::Layer;
    use tower_http::normalize_path::NormalizePathLayer;

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
        let server = TestServer::builder()
            .http_transport()
            .build(app)
            .expect("Should create test server");

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
        let server = TestServer::builder()
            .http_transport()
            .build(app)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }

    #[tokio::test]
    async fn it_should_create_and_run_with_router_wrapped_service() {
        // Build an application with a route.
        let router = Router::new()
            .route("/count", get(get_state))
            .with_state(123);
        let normalized_router = NormalizePathLayer::trim_trailing_slash().layer(router);
        let app = ServiceExt::<Request>::into_make_service(normalized_router);

        // Run the server.
        let server = TestServer::builder()
            .http_transport()
            .build(app)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}

#[cfg(test)]
mod test_into_mock_transport_layer_for_into_make_service {
    use crate::TestServer;
    use axum::Router;
    use axum::ServiceExt;
    use axum::extract::Request;
    use axum::extract::State;
    use axum::routing::get;
    use tower::Layer;
    use tower_http::normalize_path::NormalizePathLayer;

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
        let server = TestServer::builder()
            .mock_transport()
            .build(app)
            .expect("Should create test server");

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
        let server = TestServer::builder()
            .mock_transport()
            .build(app)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }

    #[tokio::test]
    async fn it_should_create_and_run_with_router_wrapped_service() {
        // Build an application with a route.
        let router = Router::new()
            .route("/count", get(get_state))
            .with_state(123);
        let normalized_router = NormalizePathLayer::trim_trailing_slash().layer(router);
        let app = ServiceExt::<Request>::into_make_service(normalized_router);

        // Run the server.
        let server = TestServer::builder()
            .mock_transport()
            .build(app)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}
