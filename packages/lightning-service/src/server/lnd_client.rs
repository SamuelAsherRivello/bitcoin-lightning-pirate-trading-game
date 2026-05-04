use super::config::ServerNodeProfile;
use crate::client::models::DemoNodeId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabObserverEvent {
    LndNodeReady(DemoNodeId),
    LndNodeUnavailable(DemoNodeId),
    PeerOnline {
        from_node: DemoNodeId,
        to_node: DemoNodeId,
    },
    PeerOffline {
        from_node: DemoNodeId,
        to_node: DemoNodeId,
    },
    ChannelPending {
        route_id: String,
    },
    ChannelActive {
        route_id: String,
    },
    ChannelClosed {
        route_id: String,
    },
    InvoiceCreated {
        node_id: DemoNodeId,
        invoice_id: String,
    },
    InvoiceSettled {
        node_id: DemoNodeId,
        invoice_id: String,
    },
    PaymentSucceeded {
        node_id: DemoNodeId,
        payment_id: String,
    },
    PaymentFailed {
        node_id: DemoNodeId,
        payment_id: String,
        reason: String,
    },
    BlockHeightChanged(u64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabObserverSource {
    LndSubscribeState,
    LndSubscribePeerEvents,
    LndSubscribeChannelEvents,
    LndSubscribeInvoices,
    LndTrackPaymentV2,
    LndChainNotifier,
    PolarHealthFallback,
}

#[derive(Clone, Debug)]
pub struct LndClientProfile {
    pub node: ServerNodeProfile,
}

impl LndClientProfile {
    pub fn new(node: ServerNodeProfile) -> Self {
        Self { node }
    }
}

#[derive(Clone, Debug)]
pub struct LndClient {
    profile: LndClientProfile,
}

impl LndClient {
    pub fn new(profile: LndClientProfile) -> Self {
        Self { profile }
    }

    pub fn node_display_name(&self) -> &str {
        &self.profile.node.display_name
    }
}

#[cfg(feature = "lnd-grpc")]
pub mod tonic_lnd_adapter {
    use super::{LndClient, LndClientProfile};

    pub fn adapter_name() -> &'static str {
        "tonic_lnd"
    }

    pub fn create_placeholder_client(profile: LndClientProfile) -> LndClient {
        LndClient::new(profile)
    }
}
