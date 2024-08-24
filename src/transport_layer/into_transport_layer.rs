use anyhow::Result;

use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;

// mod into_make_service_tower;

mod into_make_service;
mod into_make_service_with_connect_info;
mod router;

#[cfg(feature = "shuttle")]
mod axum_service;
#[cfg(feature = "shuttle")]
mod shuttle_axum;

///
/// This exists to unify how to send mock or real messages to different services.
/// This includes differences between [`Router`](::axum::Router),
/// [`IntoMakeService`](::axum::routing::IntoMakeService),
/// and [`IntoMakeServiceWithConnectInfo`](::axum::extract::connect_info::IntoMakeServiceWithConnectInfo).
///
/// Implementing this will allow you to use the `TestServer` against other types.
///
/// **Warning**, this trait may change in a future release.
///
pub trait IntoTransportLayer: Sized {
    fn into_http_transport_layer(
        self,
        builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>>;

    fn into_mock_transport_layer(self) -> Result<Box<dyn TransportLayer>>;

    fn into_default_transport(
        self,
        _builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        self.into_mock_transport_layer()
    }
}
