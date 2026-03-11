use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;
use crate::transport_layer::TransportLayerType;
use actix_web::App;
use actix_web::HttpServer;
use actix_web::body::MessageBody;
use actix_web::dev::ServerHandle;
use actix_web::dev::ServiceFactory;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use anyhow::Context;
use anyhow::Result;
use axum::body::Body;
use http::Request;
use http::Response;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use reserve_port::ReservedPort;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::future::Future;
use std::pin::Pin;
use url::Url;

pub struct ActixWebHttpTransportLayer {
    server_handle: ServerHandle,
    #[allow(dead_code)]
    maybe_reserved_port: Option<ReservedPort>,
    url: Url,
}

impl ActixWebHttpTransportLayer {
    pub(crate) fn new<F, T, B>(factory: F, builder: TransportLayerBuilder) -> Result<Self>
    where
        F: Fn() -> App<T> + Send + Clone + 'static,
        T: ServiceFactory<
                ServiceRequest,
                Config = (),
                Response = ServiceResponse<B>,
                Error = actix_web::Error,
                InitError = (),
            > + 'static,
        B: MessageBody + 'static,
    {
        let (socket_addr, tokio_listener, maybe_reserved_port) =
            builder.tcp_listener_with_reserved_port()?;

        let std_listener = tokio_listener
            .into_std()
            .context("Failed to convert tokio TcpListener to std TcpListener")?;

        let (tx, rx) = std::sync::mpsc::channel::<ServerHandle>();

        std::thread::spawn(move || {
            actix_web::rt::System::new().block_on(async move {
                let server = HttpServer::new(factory)
                    .listen(std_listener)
                    .expect("Failed to bind actix-web server to listener")
                    .run();

                tx.send(server.handle())
                    .expect("Failed to send actix-web server handle");

                server.await.expect("Actix-web server encountered an error");
            });
        });

        let server_handle = rx
            .recv()
            .context("Failed to receive actix-web server handle")?;

        let url = format!("http://{socket_addr}").parse()?;

        Ok(Self {
            server_handle,
            maybe_reserved_port,
            url,
        })
    }
}

impl TransportLayer for ActixWebHttpTransportLayer {
    fn send<'a>(
        &'a self,
        request: Request<Body>,
    ) -> Pin<Box<dyn 'a + Future<Output = Result<Response<Body>>> + Send>> {
        Box::pin(async {
            let client = Client::builder(TokioExecutor::new()).build_http();
            let hyper_response = client.request(request).await?;

            let (parts, response_body) = hyper_response.into_parts();
            let returned_response: Response<Body> =
                Response::from_parts(parts, Body::new(response_body));

            Ok(returned_response)
        })
    }

    fn url(&self) -> Option<&Url> {
        Some(&self.url)
    }

    fn transport_layer_type(&self) -> TransportLayerType {
        TransportLayerType::Http
    }

    fn is_running(&self) -> bool {
        true
    }
}

impl Debug for ActixWebHttpTransportLayer {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "ActixWebHttpTransportLayer {{ url: {} }}", self.url)
    }
}

impl Drop for ActixWebHttpTransportLayer {
    fn drop(&mut self) {
        let handle = self.server_handle.clone();
        let _ = tokio::runtime::Handle::try_current()
            .map(|rt| rt.spawn(async move { handle.stop(false).await }));
    }
}
