use ::std::fmt::Debug;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::url::Url;

use crate::internals::TransportLayer;

pub trait InnerServer: Debug {
    /// Returns the local web address for the test server.
    fn server_address<'a>(&'a self) -> &'a str;
    fn url(&self) -> Url;
    fn transport(&self) -> Arc<Mutex<Box<dyn TransportLayer>>>;
}
