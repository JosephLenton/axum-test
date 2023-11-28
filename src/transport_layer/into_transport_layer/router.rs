use ::anyhow::Result;
use ::axum::Router;
use ::async_trait::async_trait;

use super::IntoTransportLayer;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;

#[async_trait]
impl IntoTransportLayer for Router<()> {
    async fn into_http_transport_layer(
        self,
        builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        self.into_make_service().into_http_transport_layer(builder).await
    }

    async fn into_mock_transport_layer(self) -> Result<Box<dyn TransportLayer>> {
        self.into_make_service().into_mock_transport_layer().await
    }
}

#[cfg(test)]
mod test_into_http_transport_layer {
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
            transport: Some(Transport::HttpRandomPort),
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).await.expect("Should create test server");

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
            transport: Some(Transport::HttpRandomPort),
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).await.expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}

#[cfg(test)]
mod test_into_mock_transport_layer_for_router {
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
            transport: Some(Transport::MockHttp),
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).await.expect("Should create test server");

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
            transport: Some(Transport::MockHttp),
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).await.expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}
