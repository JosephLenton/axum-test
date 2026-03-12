use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerType;
use crate::util::ServeHandle;
use anyhow::Result;
use axum::body::Body;
use http::Request;
use http::Response;
use http::Uri;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use reserve_port::ReservedPort;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug)]
pub struct HttpTransportLayer {
    #[allow(dead_code)]
    serve_handle: ServeHandle,

    #[allow(dead_code)]
    maybe_reserved_port: Option<ReservedPort>,

    url: Uri,
}

impl HttpTransportLayer {
    pub(crate) fn new(
        serve_handle: ServeHandle,
        maybe_reserved_port: Option<ReservedPort>,
        url: Uri,
    ) -> Self {
        Self {
            serve_handle,
            maybe_reserved_port,
            url,
        }
    }
}

impl TransportLayer for HttpTransportLayer {
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

    fn uri(&self) -> &Uri {
        &self.url
    }

    fn transport_layer_type(&self) -> TransportLayerType {
        TransportLayerType::Http
    }

    fn is_running(&self) -> bool {
        !self.serve_handle.is_finished()
    }
}
