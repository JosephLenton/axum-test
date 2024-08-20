use ::anyhow::Result;
use ::axum::Router;
use ::shuttle_axum::AxumService;

use super::IntoTransportLayer;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;

impl IntoTransportLayer for AxumService {
    fn into_http_transport_layer(
        self,
        builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        Router::into_http_transport_layer(self.0, builder)
    }

    fn into_mock_transport_layer(self) -> Result<Box<dyn TransportLayer>> {
        Router::into_mock_transport_layer(self.0)
    }
}

#[cfg(test)]
mod test_into_http_transport_layer_for_axum_service {
    use super::*;

    use ::axum::extract::State;
    use ::axum::routing::get;
    use ::axum::Router;

    use crate::TestServerConfig;

    async fn get_state(State(count): State<u32>) -> String {
        format!("count is {}", count)
    }

    #[tokio::test]
    async fn it_should_run() {
        // Build an application with a route.
        let app: AxumService = Router::new()
            .route("/count", get(get_state))
            .with_state(123)
            .into();

        // Run the server.
        let server = TestServerConfig::builder()
            .http_transport()
            .build_server(app)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}

#[cfg(test)]
mod test_into_mock_transport_layer_for_axum_service {
    use super::*;

    use ::axum::extract::State;
    use ::axum::routing::get;
    use ::axum::Router;

    use crate::TestServerConfig;

    async fn get_state(State(count): State<u32>) -> String {
        format!("count is {}", count)
    }

    #[tokio::test]
    async fn it_should_run() {
        // Build an application with a route.
        let app: AxumService = Router::new()
            .route("/count", get(get_state))
            .with_state(123)
            .into();

        // Run the server.
        let server = TestServerConfig::builder()
            .mock_transport()
            .build_server(app)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}
