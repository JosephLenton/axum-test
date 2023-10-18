use ::anyhow::Result;
use ::async_trait::async_trait;
use ::bytes::Bytes;
use ::http::response::Parts;
use ::http::Request;
use ::hyper::Body;
use ::std::fmt::Debug;
use ::url::Url;

#[async_trait]
pub trait TransportLayer: Debug {
    async fn send(&mut self, request: Request<Body>) -> Result<(Parts, Bytes)>;
    fn url<'a>(&'a self) -> Option<&'a Url> {
        None
    }
}
