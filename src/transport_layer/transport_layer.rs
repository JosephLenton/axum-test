use ::anyhow::Result;
use ::async_trait::async_trait;
use ::http::Request;
use ::hyper::Body;
use ::std::fmt::Debug;
use bytes::Bytes;
use http::response::Parts;

#[async_trait]
pub trait TransportLayer: Debug {
    async fn send(&mut self, request: Request<Body>) -> Result<(Parts, Bytes)>;
}
