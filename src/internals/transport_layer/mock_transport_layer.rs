use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerType;
use anyhow::Error as AnyhowError;
use anyhow::Result;
use axum::body::Body;
use axum::response::Response as AxumResponse;
use bytes::Bytes;
use http::Request;
use http::Response;
use http::Uri;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use tower::Service;
use tower::util::ServiceExt;

pub struct MockTransportLayer<S> {
    service: S,
}

impl<S, RouterService> MockTransportLayer<S>
where
    S: Service<Request<Body>, Response = RouterService> + Clone + Send + Sync,
    AnyhowError: From<S::Error>,
    S::Future: Send,
    RouterService: Service<Request<Body>, Response = AxumResponse>,
{
    pub(crate) fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S, RouterService> TransportLayer for MockTransportLayer<S>
where
    S: Service<Request<Body>, Response = RouterService> + Clone + Send + Sync + 'static,
    AnyhowError: From<S::Error>,
    S::Future: Send + Sync,
    RouterService: Service<Request<Body>, Response = AxumResponse> + Send,
    RouterService::Future: Send,
    AnyhowError: From<RouterService::Error>,
{
    fn send<'a>(
        &'a self,
        mut request: Request<Body>,
    ) -> Pin<Box<dyn 'a + Future<Output = Result<Response<Body>>> + Send>> {
        Box::pin(async {
            let body: Body = Bytes::new().into();
            let empty_request = Request::builder()
                .body(body)
                .expect("should build empty request");

            let service = self.service.clone();
            let router = service.oneshot(empty_request).await?;

            if let Some(cleaned_uri) = clean_uri(request.uri()) {
                *request.uri_mut() = cleaned_uri;
            }

            let response = router.oneshot(request).await?;
            Ok(response)
        })
    }

    fn transport_layer_type(&self) -> TransportLayerType {
        TransportLayerType::Mock
    }

    /// This will always return true.
    #[inline(always)]
    fn is_running(&self) -> bool {
        true
    }
}

impl<S> Debug for MockTransportLayer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MockTransportLayer {{ service: {{unknown}} }}")
    }
}

/// On mock transport, remove the scheme and authority from the URI.
/// This is because in Axum, these are always missing in a normal server.
///
/// See: https://github.com/JosephLenton/axum-test/issues/175
fn clean_uri(uri: &Uri) -> Option<Uri> {
    if uri.scheme().is_none() && uri.authority().is_none() {
        return None;
    }

    if let Some(path_and_query) = uri.path_and_query() {
        return Some(
            Uri::builder()
                .path_and_query(path_and_query.to_owned())
                .build()
                .unwrap(),
        );
    }

    Some(Uri::default())
}
