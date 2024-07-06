use ::anyhow::Error as AnyhowError;
use ::anyhow::Result;
use ::axum::body::Body;
use ::axum::Router;
use ::bytes::Bytes;
use ::http::Request;
use ::http::Response;
use ::std::fmt::Debug;
use ::std::future::Future;
use ::std::pin::Pin;
use ::tower::util::ServiceExt;
use ::tower::Service;

use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerType;

pub struct MockTransportLayer<S> {
    service: S,
}

impl<S> MockTransportLayer<S>
where
    S: Service<Request<Body>, Response = Router> + Clone + Send + Sync,
    AnyhowError: From<S::Error>,
    S::Future: Send,
{
    pub(crate) fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S> TransportLayer for MockTransportLayer<S>
where
    S: Service<Request<Body>, Response = Router> + Clone + Send + Sync,
    AnyhowError: From<S::Error>,
    S::Future: Send,
{
    fn send<'a>(
        &'a self,
        request: Request<Body>,
    ) -> Pin<Box<dyn 'a + Future<Output = Result<Response<Body>>>>> {
        Box::pin(async {
            let body: Body = Bytes::new().into();
            let empty_request = Request::builder()
                .body(body)
                .expect("should build empty request");

            let service = self.service.clone();
            let router = service.oneshot(empty_request).await?;

            let response = router.oneshot(request).await?;

            Ok(response)
        })
    }

    fn get_type(&self) -> TransportLayerType {
        TransportLayerType::Mock
    }
}

impl<S> Debug for MockTransportLayer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MockTransportLayer {{ service: {{unknown}} }}")
    }
}
