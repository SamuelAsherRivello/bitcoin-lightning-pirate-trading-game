use serde::{Deserialize, Serialize};

use crate::client::error::LightningError;
use crate::client::models::{DemoNodeId, PolarConnectionProfile};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ServerLabProfile {
    pub network_name: String,
    pub nodes: Vec<ServerNodeProfile>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ServerNodeProfile {
    pub node_id: DemoNodeId,
    pub display_name: String,
    pub lnd_endpoint: String,
    pub tls_cert_path_or_pem: String,
    pub macaroon_path_or_hex: String,
    pub network: String,
}

pub fn load_server_lab_profile() -> Result<ServerLabProfile, LightningError> {
    platform::load_server_lab_profile()
}

pub fn validate_server_lab_profile(profile: &ServerLabProfile) -> Result<(), LightningError> {
    for required_node in DemoNodeId::ALL {
        if !profile
            .nodes
            .iter()
            .any(|node| node.node_id == required_node)
        {
            return Err(LightningError::MissingRequiredNodes);
        }
    }

    for node in &profile.nodes {
        if !node.network.eq_ignore_ascii_case("regtest") {
            return Err(LightningError::NonRegtestProfile);
        }

        if !is_local_endpoint(&node.lnd_endpoint) {
            return Err(LightningError::NonLocalEndpoint);
        }
    }

    Ok(())
}

pub fn server_lab_profile_from_polar_connection(
    network_name: String,
    connection: &PolarConnectionProfile,
) -> ServerLabProfile {
    ServerLabProfile {
        network_name,
        nodes: DemoNodeId::ALL
            .into_iter()
            .map(|node_id| {
                let node = connection.node(node_id);

                ServerNodeProfile {
                    node_id,
                    display_name: node_id.label().to_ascii_lowercase(),
                    lnd_endpoint: node.lnd_endpoint.trim().to_string(),
                    tls_cert_path_or_pem: node.tls_cert_path_or_pem.trim().to_string(),
                    macaroon_path_or_hex: node.macaroon_path_or_hex.trim().to_string(),
                    network: "regtest".to_string(),
                }
            })
            .collect(),
    }
}

pub fn validate_polar_connection_profile(
    network_name: String,
    connection: &PolarConnectionProfile,
) -> Result<ServerLabProfile, LightningError> {
    if !connection.is_complete() {
        return Err(LightningError::MissingPolarConnectionValues);
    }

    let profile = server_lab_profile_from_polar_connection(network_name, connection);
    validate_server_lab_profile(&profile)?;
    Ok(profile)
}

fn is_local_endpoint(endpoint: &str) -> bool {
    let normalized = endpoint
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("tcp://")
        .to_ascii_lowercase();

    normalized.starts_with("127.0.0.1:")
        || normalized.starts_with("localhost:")
        || normalized.starts_with("[::1]:")
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::fs;
    use std::path::PathBuf;

    use crate::client::error::LightningError;

    use super::ServerLabProfile;

    pub fn load_server_lab_profile() -> Result<ServerLabProfile, LightningError> {
        let path = profile_path();
        let value = fs::read_to_string(&path).map_err(|error| {
            LightningError::ConfigLoadFailed(format!("{} ({error})", path.display()))
        })?;
        serde_json::from_str(&value)
            .map_err(|error| LightningError::ConfigLoadFailed(error.to_string()))
    }

    fn profile_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("data")
            .join("lightning-lab-profile.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_profile() -> ServerLabProfile {
        ServerLabProfile {
            network_name: "Bitcoin Lightning Pirate Trading Game".to_string(),
            nodes: DemoNodeId::ALL
                .into_iter()
                .enumerate()
                .map(|(index, node_id)| ServerNodeProfile {
                    node_id,
                    display_name: node_id.label().to_ascii_lowercase(),
                    lnd_endpoint: format!("127.0.0.1:1000{}", index + 1),
                    tls_cert_path_or_pem: "test-cert".to_string(),
                    macaroon_path_or_hex: "test-macaroon".to_string(),
                    network: "regtest".to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn accepts_local_regtest_profiles() {
        let profile = valid_profile();

        assert!(validate_server_lab_profile(&profile).is_ok());
    }

    #[test]
    fn rejects_non_regtest_profiles() {
        let mut profile = valid_profile();
        profile.nodes[0].network = "mainnet".to_string();

        assert!(matches!(
            validate_server_lab_profile(&profile),
            Err(LightningError::NonRegtestProfile)
        ));
    }

    #[test]
    fn rejects_hosted_endpoints() {
        let mut profile = valid_profile();
        profile.nodes[0].lnd_endpoint = "lnd.example.com:10009".to_string();

        assert!(matches!(
            validate_server_lab_profile(&profile),
            Err(LightningError::NonLocalEndpoint)
        ));
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use crate::client::error::LightningError;

    use super::ServerLabProfile;

    pub fn load_server_lab_profile() -> Result<ServerLabProfile, LightningError> {
        Err(LightningError::ConfigLoadFailed(
            "Server profile files are not available in browser builds.".to_string(),
        ))
    }
}
