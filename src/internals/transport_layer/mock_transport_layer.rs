use crate::internals::ErrorMessage;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerType;
use anyhow::Error as AnyhowError;
use anyhow::Result;
use axum::body::Body;
use axum::response::Response as AxumResponse;
use bytes::Bytes;
use http::HeaderValue;
use http::Request;
use http::Response;
use http::Uri;
use http::header;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
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
                .uri(request.uri())
                .body(body)
                .expect("should build empty request");

            let service = self.service.clone();
            let router = service.oneshot(empty_request).await?;

            clean_request_for_mock(&mut request);

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
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "MockTransportLayer {{ service: {{unknown}} }}")
    }
}

fn clean_request_for_mock(request: &mut Request<Body>) {
    if let Some(authority) = request.uri().authority() {
        if !request.headers().contains_key(header::HOST) {
            let host_header = HeaderValue::from_str(authority.as_str()).error_message_fn(|| {
                format!(
                    "Failed to build HOST header from authority '{}'",
                    authority.as_str()
                )
            });

            request.headers_mut().append(header::HOST, host_header);
        }
    }

    if let Some(cleaned_uri) = clean_uri(request.uri()) {
        *request.uri_mut() = cleaned_uri;
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

#[cfg(test)]
mod test_send {
    use crate::TestServer;
    use axum::Router;
    use axum::extract::OriginalUri;
    use axum::routing::get;
    use http::HeaderMap;
    use http::header;

    async fn route_get_host_header(headers: HeaderMap) -> String {
        headers
            .get(header::HOST)
            .map(|h| h.to_str().unwrap().to_string())
            .unwrap_or_else(|| "".to_string())
    }

    async fn route_get_original_uri(original_uri: OriginalUri) -> String {
        original_uri.0.to_string()
    }

    #[tokio::test]
    async fn it_should_include_host_header_by_default() {
        let router = Router::new().route("/test", get(route_get_host_header));
        let server = TestServer::builder().mock_transport().build(router);

        server.get("/test").await.assert_text("localhost");
    }

    #[tokio::test]
    async fn it_should_not_include_scheme_or_authority_in_uri() {
        let router = Router::new().route("/uri", get(route_get_original_uri));
        let server = TestServer::builder().mock_transport().build(router);

        server.get("/uri").await.assert_text("/uri");
    }

    #[tokio::test]
    async fn it_should_have_host_header_that_matches_http_transport() {
        let router = Router::new().route("/test", get(route_get_host_header));
        let http_server = TestServer::builder().http_transport().build(router.clone());
        let http_server_address = http_server
            .server_address()
            .unwrap()
            .authority()
            .to_string();
        let expected = http_server.get("/test").await.assert_status_ok().text();

        TestServer::builder()
            .mock_transport()
            .build(router)
            .get(&format!("http://{http_server_address}/test"))
            .await
            .assert_text(expected);
    }

    #[tokio::test]
    async fn it_should_have_original_uri_that_matches_http_transport() {
        let router = Router::new().route("/uri", get(route_get_original_uri));
        let expected = TestServer::builder()
            .http_transport()
            .build(router.clone())
            .get("/uri")
            .await
            .assert_status_ok()
            .text();

        let server = TestServer::builder().mock_transport().build(router);
        server.get("/uri").await.assert_text(expected);
    }
}
