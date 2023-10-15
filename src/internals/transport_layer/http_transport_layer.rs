use ::anyhow::Result;
use ::async_trait::async_trait;
use ::bytes::Bytes;
use ::http::response::Parts;
use ::http::Request;
use ::hyper::body::to_bytes;
use ::hyper::Body;
use ::hyper::Client;
use ::tokio::task::JoinHandle;

use crate::transport_layer::TransportLayer;

#[derive(Debug)]
pub struct HttpTransportLayer {
    server_handle: JoinHandle<()>,
}

impl HttpTransportLayer {
    pub(crate) fn new(server_handle: JoinHandle<()>) -> Self {
        Self { server_handle }
    }
}

#[async_trait]
impl TransportLayer for HttpTransportLayer {
    async fn send(&mut self, request: Request<Body>) -> Result<(Parts, Bytes)> {
        let hyper_response = Client::new().request(request).await?;

        let (parts, response_body) = hyper_response.into_parts();
        let response_bytes = to_bytes(response_body).await?;

        Ok((parts, response_bytes))
    }
}

impl Drop for HttpTransportLayer {
    fn drop(&mut self) {
        self.server_handle.abort()
    }
}
