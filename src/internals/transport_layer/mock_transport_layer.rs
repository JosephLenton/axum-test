use ::anyhow::Error as AnyhowError;
use ::anyhow::Result;
use ::async_trait::async_trait;
use ::http::Request;
use ::hyper::Body;
use ::std::fmt::Debug;

use super::TransportLayer;

use axum::Router;
use bytes::Bytes;
use http::response::Parts;
use hyper::body::to_bytes;
use tower::util::ServiceExt;
use tower::Service;

pub struct MockTransportLayer<S> {
    service: S,
}

impl<S> MockTransportLayer<S>
where
    S: Service<Request<Body>, Response = Router> + Clone + Send,
    AnyhowError: From<S::Error>,
    S::Future: Send,
{
    pub(crate) fn new(service: S) -> Self {
        Self { service }
    }
}

#[async_trait]
impl<S> TransportLayer for MockTransportLayer<S>
where
    S: Service<Request<Body>, Response = Router> + Clone + Send,
    AnyhowError: From<S::Error>,
    S::Future: Send,
{
    async fn send(&mut self, request: Request<Body>) -> Result<(Parts, Bytes)> {
        let body: Body = Bytes::new().into();
        let empty_request = Request::builder()
            .body(body)
            .expect("should build empty request");

        let service = self.service.clone();
        let router = service.oneshot(empty_request).await?;

        let response = router.oneshot(request).await?;
        let (parts, response_body) = response.into_parts();
        let response_bytes = to_bytes(response_body).await?;

        Ok((parts, response_bytes))
    }
}

impl<S> Debug for MockTransportLayer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MockTransportLayer {{ service: {{unknown}} }}")
    }
}
