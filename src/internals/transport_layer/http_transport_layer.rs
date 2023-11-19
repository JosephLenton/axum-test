use ::anyhow::Result;
use ::async_trait::async_trait;
use ::bytes::Bytes;
use ::http::response::Parts;
use ::http::Request;
use ::hyper::body::to_bytes;
use ::hyper::Body;
use ::hyper::Client;
use ::reserve_port::ReservedPort;
use ::tokio::task::JoinHandle;
use ::url::Url;

use crate::transport_layer::TransportLayer;

#[derive(Debug)]
pub struct HttpTransportLayer {
    server_handle: JoinHandle<()>,

    /// If this has reserved a port for the test,
    /// then it is stored here.
    ///
    /// It's stored here until we `Drop` (as it's reserved).
    #[allow(dead_code)]
    maybe_reserved_port: Option<ReservedPort>,

    url: Url,
}

impl HttpTransportLayer {
    pub(crate) fn new(
        server_handle: JoinHandle<()>,
        maybe_reserved_port: Option<ReservedPort>,
        url: Url,
    ) -> Self {
        Self {
            server_handle,
            maybe_reserved_port,
            url,
        }
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

    fn url<'a>(&'a self) -> Option<&'a Url> {
        Some(&self.url)
    }
}

impl Drop for HttpTransportLayer {
    fn drop(&mut self) {
        self.server_handle.abort()
    }
}
