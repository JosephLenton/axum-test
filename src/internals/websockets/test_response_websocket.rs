use ::hyper::upgrade::OnUpgrade;

use crate::transport_layer::TransportLayerType;

#[derive(Clone, Debug)]
pub struct TestResponseWebSocket {
    pub maybe_on_upgrade: Option<OnUpgrade>,
    pub transport_type: TransportLayerType,
}
