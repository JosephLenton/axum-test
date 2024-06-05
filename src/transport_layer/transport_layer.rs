use ::anyhow::Result;
use ::axum::body::Body;
use ::bytes::Bytes;
use ::http::response::Parts;
use ::http::Request;
use ::std::fmt::Debug;
use ::url::Url;
use ::std::future::Future;
use ::std::pin::Pin;

pub trait TransportLayer: Debug + Send {
    fn send<'a>(&'a self, request: Request<Body>) -> Pin<Box<dyn 'a + Future<Output = Result<(Parts, Bytes)>>>>;
    fn url<'a>(&'a self) -> Option<&'a Url> {
        None
    }
}
