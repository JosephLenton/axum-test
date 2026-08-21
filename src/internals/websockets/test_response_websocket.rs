use crate::transport_layer::TransportLayerType;
use hyper::upgrade::OnUpgrade;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TestResponseWebSocket {
    pub maybe_on_upgrade: Option<OnUpgrade>,
    pub transport_type: TransportLayerType,
    pub receive_timeout: Duration,
}
