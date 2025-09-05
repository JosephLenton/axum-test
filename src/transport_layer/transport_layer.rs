use anyhow::Result;
use axum::body::Body;
use http::Request;
use http::Response;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use url::Url;

use crate::transport_layer::TransportLayerType;

pub trait TransportLayer: Debug + Send + Sync + 'static {
    fn send<'a>(
        &'a self,
        request: Request<Body>,
    ) -> Pin<Box<dyn 'a + Future<Output = Result<Response<Body>>> + Send>>;

    fn url(&self) -> Option<&Url> {
        None
    }

    fn transport_layer_type(&self) -> TransportLayerType;

    fn is_running(&self) -> bool;
}

#[cfg(test)]
mod test_sync {
    use super::*;
    use tokio::sync::OnceCell;

    #[test]
    fn it_should_compile_with_tokyo_once_cell() {
        // if it compiles, it works!
        fn _take_tokio_once_cell<T>(layer: T) -> OnceCell<Box<dyn TransportLayer>>
        where
            T: TransportLayer,
        {
            OnceCell::new_with(Some(Box::new(layer)))
        }
    }
}
