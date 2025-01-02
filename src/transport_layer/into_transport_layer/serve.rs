use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use axum::extract::Request;
use axum::response::Response;
use axum::serve::IncomingStream;
use axum::serve::Listener;
use axum::serve::Serve;
use std::convert::Infallible;
use tokio::spawn;
use tower::Service;
use url::Url;

use crate::internals::HttpTransportLayer;
use crate::transport_layer::IntoTransportLayer;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;
use crate::util::ServeHandle;

impl<L, M, S> IntoTransportLayer for Serve<L, M, S>
where
    L: Listener,
    M: for<'a> Service<IncomingStream<'a, L>, Error = Infallible, Response = S>,
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send,
{
    fn into_http_transport_layer(
        self,
        _builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        Err(anyhow!("`Serve` must be started with http or mock transport. Do not set any transport on `TestServerConfig`."))
    }

    fn into_mock_transport_layer(self) -> Result<Box<dyn TransportLayer>> {
        Err(anyhow!("`Serve` cannot be mocked, as it's underlying implementation requires a real connection. Do not set any transport on `TestServerConfig`."))
    }

    fn into_default_transport(
        self,
        _builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        let socket_addr = self.local_addr()?;

        let join_handle = spawn(async move {
            self.await
                .context("Failed to create ::axum::Server for TestServer")
                .expect("Expect server to start serving");
        });

        let server_address = format!("http://{socket_addr}");
        let server_url: Url = server_address.parse()?;

        Ok(Box::new(HttpTransportLayer::new(
            ServeHandle::new(join_handle),
            None,
            server_url,
        )))
    }
}

#[cfg(test)]
mod test_into_http_transport_layer {
    use crate::util::new_random_tokio_tcp_listener;
    use crate::TestServer;
    use axum::routing::get;
    use axum::routing::IntoMakeService;
    use axum::serve;
    use axum::Router;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_when_run_with_http() {
        // Build an application with a route.
        let app: IntoMakeService<Router> = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();
        let port = new_random_tokio_tcp_listener().unwrap();
        let application = serve(port, app);

        // Run the server.
        TestServer::builder()
            .http_transport()
            .build(application)
            .expect("Should create test server");
    }
}

#[cfg(test)]
mod test_into_mock_transport_layer {
    use crate::util::new_random_tokio_tcp_listener;
    use crate::TestServer;
    use axum::routing::get;
    use axum::routing::IntoMakeService;
    use axum::serve;
    use axum::Router;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_when_run_with_mock_http() {
        // Build an application with a route.
        let app: IntoMakeService<Router> = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();
        let port = new_random_tokio_tcp_listener().unwrap();
        let application = serve(port, app);

        // Run the server.
        TestServer::builder()
            .mock_transport()
            .build(application)
            .expect("Should create test server");
    }
}

#[cfg(test)]
mod test_into_default_transport {
    use crate::util::new_random_tokio_tcp_listener;
    use crate::TestServer;
    use axum::routing::get;
    use axum::routing::IntoMakeService;
    use axum::serve;
    use axum::Router;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    async fn it_should_run_service() {
        // Build an application with a route.
        let app: IntoMakeService<Router> = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();
        let port = new_random_tokio_tcp_listener().unwrap();
        let application = serve(port, app);

        // Run the server.
        let server = TestServer::builder()
            .build(application)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }
}
