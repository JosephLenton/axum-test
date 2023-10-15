use ::std::fmt::Debug;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::url::Url;

use super::InnerServer;
use crate::transport_layer::IntoMockTransportLayer;
use crate::transport_layer::TransportLayer;

pub struct InnerMockServer {
    transport: Arc<Mutex<Box<dyn TransportLayer>>>,
}

impl InnerMockServer {
    pub fn new<A>(app: A) -> Self
    where
        A: IntoMockTransportLayer,
    {
        Self {
            transport: Arc::new(Mutex::new(app.into_mock_transport_layer())),
        }
    }
}

impl InnerServer for InnerMockServer {
    /// Returns 'example.com'.
    fn server_address<'a>(&'a self) -> &'a str {
        &"http://example.com"
    }

    fn url(&self) -> Url {
        "http://example.com".parse().unwrap()
    }

    fn transport(&self) -> Arc<Mutex<Box<dyn TransportLayer>>> {
        self.transport.clone()
    }
}

impl Debug for InnerMockServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InnerMockServer {{ service: {{unknown}} }}")
    }
}
