use anyhow::Result;
use shuttle_axum::ShuttleAxum;

use crate::transport_layer::IntoTransportLayer;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;

impl IntoTransportLayer for ShuttleAxum {
    fn into_http_transport_layer(
        self,
        builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        self.map_err(Into::into)
            .and_then(|axum_service| axum_service.into_http_transport_layer(builder))
    }

    fn into_mock_transport_layer(self) -> Result<Box<dyn TransportLayer>> {
        self.map_err(Into::into)
            .and_then(|axum_service| axum_service.into_mock_transport_layer())
    }
}

#[cfg(test)]
mod test_into_http_transport_layer_for_shuttle_axum {
    use super::*;

    use axum::Router;
    use axum::extract::State;
    use axum::routing::get;
    use shuttle_axum::AxumService;

    use crate::TestServer;

    async fn get_state(State(count): State<u32>) -> String {
        format!("count is {}", count)
    }

    #[tokio::test]
    async fn it_should_run() {
        // Build an application with a route.
        let router = Router::new()
            .route("/count", get(get_state))
            .with_state(123);
        let app: ShuttleAxum = Ok(AxumService::from(router));

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
mod test_into_mock_transport_layer_for_shuttle_axum {
    use super::*;

    use axum::Router;
    use axum::extract::State;
    use axum::routing::get;
    use shuttle_axum::AxumService;

    use crate::TestServer;

    async fn get_state(State(count): State<u32>) -> String {
        format!("count is {}", count)
    }

    #[tokio::test]
    async fn it_should_run() {
        // Build an application with a route.
        let router = Router::new()
            .route("/count", get(get_state))
            .with_state(123);
        let app: ShuttleAxum = Ok(AxumService::from(router));

        // Run the server.
        let server = TestServer::builder()
            .mock_transport()
            .build(app)
            .expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}
