use crate::internals::HttpTransportLayer;
use crate::transport_layer::IntoTransportLayer;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;
use crate::util::ServeHandle;
use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use axum::extract::Request;
use axum::response::Response;
use axum::serve::IncomingStream;
use axum::serve::WithGracefulShutdown;
use std::convert::Infallible;
use std::future::Future;
use tokio::net::TcpListener;
use tokio::spawn;
use tower::Service;
use url::Url;

impl<M, S, F> IntoTransportLayer for WithGracefulShutdown<TcpListener, M, S, F>
where
    M: for<'a> Service<IncomingStream<'a, TcpListener>, Error = Infallible, Response = S>
        + Send
        + 'static,
    for<'a> <M as Service<IncomingStream<'a, TcpListener>>>::Future: Send,
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send,
    F: Future<Output = ()> + Send + 'static,
{
    fn into_http_transport_layer(
        self,
        _builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        Err(anyhow!(
            "`WithGracefulShutdown` must be started with http or mock transport. Do not set any transport on `TestServerConfig`."
        ))
    }

    fn into_mock_transport_layer(self) -> Result<Box<dyn TransportLayer>> {
        Err(anyhow!(
            "`WithGracefulShutdown` cannot be mocked, as it's underlying implementation requires a real connection. Do not set any transport on `TestServerConfig`."
        ))
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
    use crate::TestServer;
    use crate::util::new_random_tokio_tcp_listener;
    use axum::Router;
    use axum::routing::IntoMakeService;
    use axum::routing::get;
    use axum::serve;
    use std::future::pending;

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
        let application = serve(port, app).with_graceful_shutdown(pending());

        // Run the server.
        TestServer::builder()
            .http_transport()
            .build(application)
            .expect("Should create test server");
    }
}

#[cfg(test)]
mod test_into_mock_transport_layer {
    use crate::TestServer;
    use crate::util::new_random_tokio_tcp_listener;
    use axum::Router;
    use axum::routing::IntoMakeService;
    use axum::routing::get;
    use axum::serve;
    use std::future::pending;

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
        let application = serve(port, app).with_graceful_shutdown(pending());

        // Run the server.
        TestServer::builder()
            .mock_transport()
            .build(application)
            .expect("Should create test server");
    }
}

#[cfg(test)]
mod test_into_default_transport {
    use crate::TestServer;
    use crate::util::new_random_tokio_tcp_listener;
    use axum::Router;
    use axum::routing::IntoMakeService;
    use axum::routing::get;
    use axum::serve;
    use std::future::pending;

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
        let application = serve(port, app).with_graceful_shutdown(pending());

        // Run the server.
        let server = TestServer::builder()
            .build(application)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }
}
