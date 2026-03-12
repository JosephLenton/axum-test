use crate::internals::ActixWebMockTransportLayer;
use crate::internals::HttpTransportLayer;
use crate::transport_layer::IntoTransportLayer;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;
use crate::util::spawn_actix_serve;
use actix_web::App;
use actix_web::body::BoxBody;
use actix_web::dev::ServiceFactory;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use anyhow::Result;

impl<F, T> IntoTransportLayer for F
where
    F: Fn() -> App<T> + Send + Clone + 'static,
    T: ServiceFactory<
            ServiceRequest,
            Config = (),
            Response = ServiceResponse<BoxBody>,
            Error = actix_web::Error,
            InitError = (),
        > + 'static,
{
    fn into_http_transport_layer(
        self,
        builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        let (socket_addr, tcp_listener, maybe_reserved_port) =
            builder.tcp_listener_with_reserved_port()?;

        let serve_handle = spawn_actix_serve(tcp_listener, self);
        let server_address = format!("http://{socket_addr}");
        let server_url = server_address.parse()?;

        Ok(Box::new(HttpTransportLayer::new(
            serve_handle,
            maybe_reserved_port,
            server_url,
        )))
    }

    fn into_mock_transport_layer(self) -> Result<Box<dyn TransportLayer>> {
        Ok(Box::new(ActixWebMockTransportLayer::new(self)))
    }

    fn into_default_transport(
        self,
        _builder: TransportLayerBuilder,
    ) -> Result<Box<dyn TransportLayer>> {
        self.into_mock_transport_layer()
    }
}
