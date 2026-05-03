use super::config::ServerNodeProfile;

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
