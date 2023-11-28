use ::anyhow::Result;
use ::async_trait::async_trait;

use crate::transport_layer::TransportLayer;

mod into_make_service_with_connect_info;
pub use self::into_make_service_with_connect_info::*;

mod into_make_service;
pub use self::into_make_service::*;

mod router;
pub use self::router::*;
use super::TransportLayerBuilder;

///
/// This exists to unify how to send mock or real messages to different services.
/// This includes differences between [`Router`](::axum::routing::Router),
/// [`IntoMakeService`](::axum::routing::IntoMakeService),
/// and [`IntoMakeServiceWithConnectInfo`](::axum::extract::connect_info::IntoMakeServiceWithConnectInfo).
///
/// Implementing this will allow you to use the `TestServer` against other types.
///
/// **Warning**, this trait may change in a future release.
///
#[async_trait]
pub trait IntoTransportLayer: Sized {
    async fn into_http_transport_layer(
        self,
        builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>>;

    async fn into_mock_transport_layer(self) -> Result<Box<dyn TransportLayer>>;

    async fn into_default_transport(
        self,
        _builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        self.into_mock_transport_layer().await
    }
}
