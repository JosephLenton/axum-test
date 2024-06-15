use super::TransportLayerType;
use ::anyhow::Result;
use ::axum::body::Body;
use ::http::Request;
use ::http::Response;
use ::std::fmt::Debug;
use ::std::future::Future;
use ::std::pin::Pin;
use ::url::Url;

pub trait TransportLayer: Debug + Send {
    fn send<'a>(
        &'a self,
        request: Request<Body>,
    ) -> Pin<Box<dyn 'a + Future<Output = Result<Response<Body>>>>>;

    fn url<'a>(&'a self) -> Option<&'a Url> {
        None
    }

    fn get_type(&self) -> TransportLayerType;
}
