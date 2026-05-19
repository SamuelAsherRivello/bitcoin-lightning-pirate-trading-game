use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const DEFAULT_NETWORK_NAME: &str = "Dioxus Bitcoin Lightning Game";
pub const DEFAULT_BITCOIN_BACKEND_NAME: &str = "My Bitcoin Node";
pub const DEFAULT_SATS_PER_TRANSACTION: u64 = 1_000;
pub const MAX_SATS_PER_TRANSACTION: u64 = 100_000;
pub const DEFAULT_ROUTE_CAPACITY_SATS: u64 = 250_000;
pub const MAX_TRA_ITEMS_PER_NODE: usize = 3;
pub const BOOK_ITEM_ID: u32 = 1;
pub const APPLE_ITEM_ID: u32 = 2;
pub const GAME_TREASURY_NODE_LABEL: &str = "GAME_TREASURY";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DemoNodeId {
    GameTreasury,
    Alice,
    Bob,
    Carol,
}

impl DemoNodeId {
    pub const ALL: [Self; 3] = [Self::Alice, Self::Bob, Self::Carol];

    pub fn label(self) -> &'static str {
        match self {
            Self::Alice => "Alice",
            Self::Bob => "Bob",
            Self::Carol => "Carol",
            Self::GameTreasury => GAME_TREASURY_NODE_LABEL,
        }
    }

    pub fn role(self) -> NodeRole {
        match self {
            Self::GameTreasury => NodeRole::GameTreasury,
            Self::Alice => NodeRole::Player,
            Self::Bob => NodeRole::BeachMerchant,
            Self::Carol => NodeRole::MountainMerchant,
        }
    }

    pub fn location(self) -> Location {
        match self {
            Self::GameTreasury => Location::Town,
            Self::Alice => Location::Town,
            Self::Bob => Location::Beach,
            Self::Carol => Location::Mountain,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NodeRole {
    GameTreasury,
    Player,
    BeachMerchant,
    MountainMerchant,
}

impl NodeRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Player => "Player",
            Self::BeachMerchant => "Beach merchant",
            Self::MountainMerchant => "Mountain merchant",
            Self::GameTreasury => "Game treasury",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Location {
    Town,
    Beach,
    Mountain,
    Desert,
}

impl Location {
    pub fn label(self) -> &'static str {
        match self {
            Self::Town => "Town",
            Self::Beach => "Beach",
            Self::Mountain => "Mountain",
            Self::Desert => "Desert",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SetupMode {
    ServerConfig,
    BrowserRegtestOnly,
}

impl SetupMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::ServerConfig => "Polar Connection (Networked)",
            Self::BrowserRegtestOnly => "Mock Connection (Offline)",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConnectionStatus {
    NotConfigured,
    SavedOffline,
    Connected,
    PartiallyConnected,
    Invalid,
}

impl ConnectionStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotConfigured => "Not configured",
            Self::SavedOffline => "Saved but offline",
            Self::Connected => "Connected",
            Self::PartiallyConnected => "Partially connected",
            Self::Invalid => "Invalid",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolarNodeConnection {
    pub lnd_endpoint: String,
    pub tls_cert_path_or_pem: String,
    pub macaroon_path_or_hex: String,
}

impl PolarNodeConnection {
    pub fn is_complete(&self) -> bool {
        !self.lnd_endpoint.trim().is_empty()
            && !self.tls_cert_path_or_pem.trim().is_empty()
            && !self.macaroon_path_or_hex.trim().is_empty()
    }
}

impl Default for PolarNodeConnection {
    fn default() -> Self {
        Self {
            lnd_endpoint: String::new(),
            tls_cert_path_or_pem: String::new(),
            macaroon_path_or_hex: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PolarConnectionProfile {
    pub alice: PolarNodeConnection,
    pub bob: PolarNodeConnection,
    pub carol: PolarNodeConnection,
}

impl PolarConnectionProfile {
    pub fn node(&self, node_id: DemoNodeId) -> &PolarNodeConnection {
        match node_id {
            DemoNodeId::GameTreasury => &self.alice,
            DemoNodeId::Alice => &self.alice,
            DemoNodeId::Bob => &self.bob,
            DemoNodeId::Carol => &self.carol,
        }
    }

    pub fn is_complete(&self) -> bool {
        DemoNodeId::ALL
            .into_iter()
            .all(|node_id| self.node(node_id).is_complete())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolarAutomationProfile {
    pub bridge_url: String,
    pub network_id: String,
    pub bitcoin_backend_name: String,
}

impl PolarAutomationProfile {
    pub fn is_complete(&self) -> bool {
        !self.bridge_url.trim().is_empty()
    }

    pub fn is_local_bridge(&self) -> bool {
        Self::is_valid_local_bridge_url(&self.bridge_url)
    }

    pub fn is_valid_local_bridge_url(bridge_url: &str) -> bool {
        let Some(without_scheme) = bridge_url
            .trim()
            .to_ascii_lowercase()
            .strip_prefix("http://")
            .map(str::to_string)
        else {
            return false;
        };

        if without_scheme.contains('?') || without_scheme.contains('#') {
            return false;
        }

        let authority = without_scheme.trim_end_matches('/');
        if authority.contains('/') {
            return false;
        }

        let Some((host, port)) = authority.split_once(':') else {
            return false;
        };

        if host != "localhost" && host != "127.0.0.1" {
            return false;
        }

        port.parse::<u16>().is_ok_and(|port| port > 0)
    }
}

impl Default for PolarAutomationProfile {
    fn default() -> Self {
        Self {
            bridge_url: "http://localhost:37373".to_string(),
            network_id: String::new(),
            bitcoin_backend_name: DEFAULT_BITCOIN_BACKEND_NAME.to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SetupProfile {
    pub sats_per_transaction: u64,
    pub network_name: String,
    pub setup_mode: SetupMode,
    #[serde(default)]
    pub polar_connection: PolarConnectionProfile,
    #[serde(default)]
    pub polar_automation: PolarAutomationProfile,
    #[serde(default)]
    pub polar_block_height_confirmed: bool,
    #[serde(default)]
    pub game_treasury_ready: bool,
    #[serde(default)]
    pub game_treasury_funded_sats: u64,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub connection_status: ConnectionStatus,
}

impl SetupProfile {
    pub fn is_connected(&self) -> bool {
        self.connection_status == ConnectionStatus::Connected
    }
}

impl Default for SetupProfile {
    fn default() -> Self {
        Self {
            sats_per_transaction: DEFAULT_SATS_PER_TRANSACTION,
            network_name: DEFAULT_NETWORK_NAME.to_string(),
            setup_mode: SetupMode::ServerConfig,
            polar_connection: PolarConnectionProfile::default(),
            polar_automation: PolarAutomationProfile::default(),
            polar_block_height_confirmed: false,
            game_treasury_ready: false,
            game_treasury_funded_sats: 0,
            last_verified_at: None,
            connection_status: ConnectionStatus::NotConfigured,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NodeStatus {
    Offline,
    Online,
    Locked,
    Error,
}

impl NodeStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Offline => "Offline",
            Self::Online => "Online",
            Self::Locked => "Locked",
            Self::Error => "Error",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DemoNode {
    pub node_id: DemoNodeId,
    pub role: NodeRole,
    pub location: Location,
    pub alias: String,
    pub pubkey: Option<String>,
    pub wallet_balance_sats: u64,
    pub channel_balance_sats: u64,
    pub status: NodeStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RouteStatus {
    Missing,
    UnderConstruction,
    Active,
    Closing,
    Closed,
    Error,
}

impl RouteStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Missing => "Missing",
            Self::UnderConstruction => "Under Construction",
            Self::Active => "Active",
            Self::Closing => "Closing",
            Self::Closed => "Closed",
            Self::Error => "Error",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TradeRoute {
    pub route_id: String,
    pub from_node: DemoNodeId,
    pub to_node: DemoNodeId,
    pub game_label: String,
    pub lnd_channel_point: Option<String>,
    pub capacity_sats: u64,
    pub local_balance_sats: u64,
    pub remote_balance_sats: u64,
    pub status: RouteStatus,
    pub requires_next_block: bool,
}

impl TradeRoute {
    pub fn connects(&self, left: DemoNodeId, right: DemoNodeId) -> bool {
        (self.from_node == left && self.to_node == right)
            || (self.from_node == right && self.to_node == left)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InvoiceStatus {
    Created,
    Settled,
    Expired,
    Canceled,
    Error,
}

impl InvoiceStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Settled => "Settled",
            Self::Expired => "Expired",
            Self::Canceled => "Canceled",
            Self::Error => "Error",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InvoiceRequest {
    pub invoice_id: String,
    pub creator_node: DemoNodeId,
    pub expected_payer_node: Option<DemoNodeId>,
    pub amount_sats: u64,
    pub memo: String,
    pub payment_request: String,
    pub status: InvoiceStatus,
    pub created_at: DateTime<Utc>,
    pub settled_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PaymentStatus {
    Pending,
    Succeeded,
    Failed,
}

impl PaymentStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Succeeded => "Succeeded",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PaymentAttempt {
    pub payment_id: String,
    pub payer_node: DemoNodeId,
    pub payee_node: DemoNodeId,
    pub invoice_id: String,
    pub amount_sats: u64,
    pub route_summary: Option<String>,
    pub status: PaymentStatus,
    pub failure_reason: Option<String>,
    pub requires_block: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlockWaitReason {
    ChannelOpenConfirmation,
    ChannelCloseConfirmation,
    WalletFundingConfirmation,
}

impl BlockWaitReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::ChannelOpenConfirmation => "Channel open confirmation",
            Self::ChannelCloseConfirmation => "Channel close confirmation",
            Self::WalletFundingConfirmation => "Wallet funding confirmation",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlockWaitStatus {
    Pending,
    Mined,
    Failed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BlockWaitAction {
    pub action_id: String,
    pub reason: BlockWaitReason,
    pub affected_route_id: Option<String>,
    pub blocks_requested: u64,
    pub status: BlockWaitStatus,
    pub resulting_height: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperationFaqRow {
    pub operation: String,
    pub needs_bitcoin_node: bool,
    pub needs_mined_block: bool,
    pub plain_explanation: String,
    pub game_example: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ActionLogEntry {
    pub id: String,
    pub summary: String,
    pub network_detail: String,
    #[serde(default)]
    pub details: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TraOwnershipStatus {
    Verified,
    Pending,
    Missing,
    Unsupported,
    Failed,
}

impl TraOwnershipStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Verified => "Verified",
            Self::Pending => "Pending",
            Self::Missing => "Missing",
            Self::Unsupported => "Unsupported",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TraTransferStatus {
    None,
    Pending,
    Succeeded,
    Failed,
}

impl TraTransferStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Pending => "Pending",
            Self::Succeeded => "Succeeded",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GameItemDefinition {
    pub item_id: u32,
    pub item_type: String,
    pub display_name: String,
    pub cost_sats: u64,
    pub visual_key: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TraItem {
    pub tra_id: String,
    pub asset_id: String,
    pub unique_name: String,
    pub item_id: u32,
    pub owner_node: DemoNodeId,
    pub ownership_status: TraOwnershipStatus,
    pub transfer_status: TraTransferStatus,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MintTraRequest {
    pub owner_node: DemoNodeId,
    pub unique_name: String,
    pub item_id: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TransferTraRequest {
    pub tra_id: String,
    pub from_node: DemoNodeId,
    pub to_node: DemoNodeId,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TreasuryStatus {
    NotStarted,
    CreatingNode,
    Funding,
    CreatingItems,
    Ready,
    Refreshing,
    Degraded,
    Failed,
}

impl TreasuryStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "Not started",
            Self::CreatingNode => "Creating node",
            Self::Funding => "Funding",
            Self::CreatingItems => "Creating items",
            Self::Ready => "Ready",
            Self::Refreshing => "Refreshing",
            Self::Degraded => "Needs attention",
            Self::Failed => "Failed",
        }
    }
}

impl Default for TreasuryStatus {
    fn default() -> Self {
        Self::NotStarted
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TreasuryEntryDirection {
    Increase,
    Decrease,
    TransferOut,
    TransferIn,
    NoChange,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TreasuryResource {
    pub resource_id: String,
    pub resource_type: String,
    pub display_name: String,
    pub item_id: Option<u32>,
    pub owner: String,
    pub estimated_value_sats: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TreasuryEntry {
    pub entry_id: String,
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub direction: TreasuryEntryDirection,
    pub amount_sats: Option<u64>,
    pub item_id: Option<u32>,
    pub item_name: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub related_action: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TreasuryImpactPreview {
    pub action_label: String,
    pub can_execute: bool,
    pub blocking_reason: Option<String>,
    pub expected_sats_delta: Option<i64>,
    pub expected_item_movements: Vec<String>,
    pub requires_refresh: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NpcItemTransfer {
    pub transfer_id: String,
    pub item_id: u32,
    pub item_name: String,
    pub source: String,
    pub destination: DemoNodeId,
    pub status: TraTransferStatus,
    pub entry_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GameTreasury {
    pub node_label: String,
    pub status: TreasuryStatus,
    pub spendable_sats: u64,
    pub inventory_value_sats: u64,
    pub owned_items: Vec<TreasuryResource>,
    pub recent_entries: Vec<TreasuryEntry>,
    pub last_updated_at: Option<DateTime<Utc>>,
}

impl Default for GameTreasury {
    fn default() -> Self {
        Self {
            node_label: GAME_TREASURY_NODE_LABEL.to_string(),
            status: TreasuryStatus::NotStarted,
            spendable_sats: 0,
            inventory_value_sats: 0,
            owned_items: Vec::new(),
            recent_entries: Vec::new(),
            last_updated_at: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LabState {
    pub profile: SetupProfile,
    pub nodes: Vec<DemoNode>,
    pub trade_routes: Vec<TradeRoute>,
    pub recent_invoices: Vec<InvoiceRequest>,
    pub recent_payments: Vec<PaymentAttempt>,
    pub block_actions: Vec<BlockWaitAction>,
    #[serde(default)]
    pub tra_items: Vec<TraItem>,
    #[serde(default)]
    pub game_treasury: GameTreasury,
    #[serde(default)]
    pub npc_item_transfers: Vec<NpcItemTransfer>,
    pub operation_faq: Vec<OperationFaqRow>,
    pub block_height: u64,
    #[serde(default)]
    pub polar_observed_block_height: Option<u64>,
    pub warnings: Vec<String>,
    pub action_log: Vec<ActionLogEntry>,
}

#[cfg(test)]
mod tests {
    use super::PolarAutomationProfile;

    #[test]
    fn polar_bridge_url_accepts_localhost_and_loopback_with_ports() {
        assert!(PolarAutomationProfile::is_valid_local_bridge_url(
            "http://localhost:37373"
        ));
        assert!(PolarAutomationProfile::is_valid_local_bridge_url(
            "http://127.0.0.1:37373/"
        ));
    }

    #[test]
    fn polar_bridge_url_rejects_non_local_or_malformed_urls() {
        for bridge_url in [
            "",
            "https://localhost:37373",
            "http://localhost",
            "http://localhost:0",
            "http://localhost:not-a-port",
            "http://localhost:37373/path",
            "http://localhost:37373?debug=true",
            "http://localhost.example.com:37373",
            "http://192.168.1.10:37373",
        ] {
            assert!(
                !PolarAutomationProfile::is_valid_local_bridge_url(bridge_url),
                "{bridge_url} should be rejected"
            );
        }
    }
}
