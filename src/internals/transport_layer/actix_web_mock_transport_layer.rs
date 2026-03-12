use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerType;
use crate::util::SafeSend;
use crate::util::SafeSendBuilder;
use actix_web::App;
use actix_web::Error as ActixWebError;
use actix_web::body::BoxBody;
use actix_web::dev::ServiceFactory;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::http::Method as ActixMethod;
use actix_web::http::header::HeaderName as ActixHeaderName;
use actix_web::http::header::HeaderValue as ActixHeaderValue;
use actix_web::test::call_service;
use actix_web::test::init_service;
use anyhow::Result;
use axum::body::Body;
use http::Request;
use http::Response;
use http_body_util::BodyExt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::future::Future;
use std::pin::Pin;

pub struct ActixWebMockTransportLayer {
    handle: SafeSend<Request<Body>, Result<Response<Body>>>,
}

impl ActixWebMockTransportLayer {
    pub(crate) fn new<F, T>(factory: F) -> Self
    where
        F: Fn() -> App<T> + Send + Clone + 'static,
        T: ServiceFactory<
                ServiceRequest,
                Config = (),
                Response = ServiceResponse<BoxBody>,
                Error = ActixWebError,
                InitError = (),
            > + 'static,
    {
        let handle = SafeSendBuilder::new(async move || init_service(factory()).await).on_send(
            |service, http_request| {
                Box::pin(async move {
                    let test_req = to_test_request(http_request).await?;
                    let actix_req = test_req.to_request();
                    let actix_response = call_service(service, actix_req).await;
                    to_http_response(actix_response).await
                })
            },
        );

        Self { handle }
    }
}

async fn to_test_request(request: Request<Body>) -> Result<actix_web::test::TestRequest> {
    let (parts, body) = request.into_parts();

    let body_bytes = body
        .collect()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to collect request body: {e}"))?
        .to_bytes();

    let actix_method = ActixMethod::from_bytes(parts.method.as_str().as_bytes())
        .map_err(|e| anyhow::anyhow!("Invalid HTTP method: {e}"))?;

    let mut test_req = actix_web::test::TestRequest::default()
        .method(actix_method)
        .uri(parts.uri.to_string().as_str())
        .set_payload(body_bytes);

    for (name, value) in &parts.headers {
        let actix_name = ActixHeaderName::from_bytes(name.as_str().as_bytes())
            .map_err(|e| anyhow::anyhow!("Invalid header name: {e}"))?;
        let actix_value = ActixHeaderValue::from_bytes(value.as_bytes())
            .map_err(|e| anyhow::anyhow!("Invalid header value: {e}"))?;
        test_req = test_req.insert_header((actix_name, actix_value));
    }

    Ok(test_req)
}

async fn to_http_response(service_response: ServiceResponse<BoxBody>) -> Result<Response<Body>> {
    let status_u16 = service_response.status().as_u16();
    let headers: Vec<(String, Vec<u8>)> = service_response
        .headers()
        .iter()
        .map(|(name, value)| (name.as_str().to_owned(), value.as_bytes().to_vec()))
        .collect();

    let body = service_response.into_body();
    let body_bytes = actix_web::body::to_bytes(body)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to collect response body: {e}"))?;

    let status = http::StatusCode::from_u16(status_u16)?;
    let mut builder = Response::builder().status(status);

    for (name, value) in &headers {
        builder = builder.header(name.as_str(), value.as_slice());
    }

    Ok(builder.body(Body::from(body_bytes))?)
}

impl TransportLayer for ActixWebMockTransportLayer {
    fn send<'a>(
        &'a self,
        request: Request<Body>,
    ) -> Pin<Box<dyn 'a + Future<Output = Result<Response<Body>>> + Send>> {
        Box::pin(async move { self.handle.send(request).await? })
    }

    fn transport_layer_type(&self) -> TransportLayerType {
        TransportLayerType::Mock
    }

    fn is_running(&self) -> bool {
        self.handle.is_running()
    }
}

impl Debug for ActixWebMockTransportLayer {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "ActixWebMockTransportLayer {{ service: {{unknown}} }}")
    }
}
