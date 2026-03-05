use crate::transport_layer::TransportLayerType;
use hyper::upgrade::OnUpgrade;

#[derive(Debug, Clone)]
pub struct TestResponseWebSocket {
    pub maybe_on_upgrade: Option<OnUpgrade>,
    pub transport_type: TransportLayerType,
}
