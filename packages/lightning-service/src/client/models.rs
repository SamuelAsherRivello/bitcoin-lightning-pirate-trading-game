use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const DEFAULT_NETWORK_NAME: &str = "Dioxus Bitcoin Lightning Game";
pub const DEFAULT_BITCOIN_BACKEND_NAME: &str = "BITCOIN_TESTNET";
pub const DEFAULT_SATS_PER_TRANSACTION: u64 = 1_000;
pub const MAX_SATS_PER_TRANSACTION: u64 = 100_000;
pub const DEFAULT_ROUTE_CAPACITY_SATS: u64 = 250_000;
pub const DEFAULT_LNAUTH_BRIDGE_URL: &str = "http://localhost:37374";
pub const MAX_TRA_ITEMS_PER_NODE: usize = 3;
pub const BOOK_ITEM_ID: u32 = 1;
pub const APPLE_ITEM_ID: u32 = 2;
pub const GAME_TREASURY_NODE_LABEL: &str = "GAME_LND";

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
pub enum UserAuthMode {
    App,
    MockLnAuth,
    LnAuth,
}

impl UserAuthMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::App => "App",
            Self::MockLnAuth => "Mock LNAuth",
            Self::LnAuth => "LNAuth",
        }
    }

    pub fn is_development_only(self) -> bool {
        self == Self::App
    }

    pub fn is_mock(self) -> bool {
        self == Self::MockLnAuth
    }

    pub fn requires_player_auth(self) -> bool {
        matches!(self, Self::MockLnAuth | Self::LnAuth)
    }

    pub fn requires_authorization_event_approval(self) -> bool {
        matches!(self, Self::MockLnAuth | Self::LnAuth)
    }
}

impl Default for UserAuthMode {
    fn default() -> Self {
        Self::App
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum WalletCompatibilityStatus {
    PendingValidation,
    Validated,
    Blocked,
}

impl Default for WalletCompatibilityStatus {
    fn default() -> Self {
        Self::PendingValidation
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WalletRecommendationTip {
    pub wallet_name: String,
    pub platforms: Vec<String>,
    pub recommendation_reason: String,
    pub official_links: Vec<String>,
    pub fallback_note: Option<String>,
    pub compatibility_status: WalletCompatibilityStatus,
}

impl Default for WalletRecommendationTip {
    fn default() -> Self {
        Self {
            wallet_name: "ZEUS".to_string(),
            platforms: vec!["Android".to_string(), "iOS".to_string()],
            recommendation_reason:
                "Mainstream mobile Lightning wallet with documented LNURL auth support.".to_string(),
            official_links: vec![
                "https://zeusln.app/".to_string(),
                "https://docs.zeusln.app/".to_string(),
            ],
            fallback_note: Some(
                "Phone scans require this app's LNAuth bridge URL to be reachable from the phone."
                    .to_string(),
            ),
            compatibility_status: WalletCompatibilityStatus::PendingValidation,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlayerIdentity {
    pub linking_key_fingerprint: String,
    pub display_label: String,
    pub authenticated_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthSessionStatus {
    Created,
    Displayed,
    Approved,
    Expired,
    Rejected,
    Failed,
    Canceled,
}

impl Default for AuthSessionStatus {
    fn default() -> Self {
        Self::Created
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthAction {
    Login,
    Register,
    Link,
    Auth,
}

impl Default for AuthAction {
    fn default() -> Self {
        Self::Login
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlayerAuthSession {
    pub session_id: String,
    pub challenge_id: String,
    pub lnurl: String,
    pub qr_payload: String,
    pub action: AuthAction,
    pub status: AuthSessionStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub player_identity: Option<PlayerIdentity>,
    pub failure_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthorizationEventKind {
    PlayerLogin,
    SendSats,
    PayInvoice,
    OpenRoute,
    CloseRoute,
    TransferAsset,
    OtherValueMovingAction,
}

impl Default for AuthorizationEventKind {
    fn default() -> Self {
        Self::PlayerLogin
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthorizationRiskLevel {
    Low,
    ValueMoving,
    DurableStateChange,
}

impl Default for AuthorizationRiskLevel {
    fn default() -> Self {
        Self::Low
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AuthorizationEvent {
    pub event_id: String,
    pub event_kind: AuthorizationEventKind,
    pub summary: String,
    pub requires_qr_approval: bool,
    pub risk_level: AuthorizationRiskLevel,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ApprovalOperationKind {
    SendSats,
    PayInvoice,
    OpenRoute,
    CloseRoute,
    TransferAsset,
    ChannelFunding,
    OtherPlayerChainAction,
}

impl Default for ApprovalOperationKind {
    fn default() -> Self {
        Self::SendSats
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionApprovalStatus {
    NotRequired,
    Required,
    Pending,
    Approved,
    Expired,
    Rejected,
    Failed,
    Canceled,
}

impl Default for TransactionApprovalStatus {
    fn default() -> Self {
        Self::NotRequired
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TransactionApproval {
    pub approval_id: String,
    pub operation_kind: ApprovalOperationKind,
    pub operation_summary: String,
    pub player_identity: Option<PlayerIdentity>,
    pub amount_sats: Option<u64>,
    pub status: TransactionApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub approved_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum QrAuthorizationKind {
    Login,
    SendSats,
    NostrProfile,
}

impl Default for QrAuthorizationKind {
    fn default() -> Self {
        Self::Login
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum QrAuthorizationStatus {
    Open,
    MockCompleting,
    Approved,
    Canceled,
    Expired,
    Failed,
}

impl Default for QrAuthorizationStatus {
    fn default() -> Self {
        Self::Open
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct QrAuthorizationModal {
    pub modal_id: String,
    pub title: String,
    pub description: String,
    pub qr_payload: String,
    pub qr_kind: QrAuthorizationKind,
    pub amount_sats: Option<u64>,
    pub status: QrAuthorizationStatus,
    pub can_cancel: bool,
    pub opened_at: DateTime<Utc>,
    pub auto_complete_after_ms: Option<u64>,
}

pub const MAX_NOSTR_USERNAME_CHARS: usize = 32;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NostrIdentityStatus {
    Unauthenticated,
    PendingAuth,
    Authenticated,
    AuthFailed,
    Canceled,
}

impl Default for NostrIdentityStatus {
    fn default() -> Self {
        Self::Unauthenticated
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NostrProfileSource {
    Relay,
    LocalSnapshot,
    PendingPublish,
    Mock,
}

impl Default for NostrProfileSource {
    fn default() -> Self {
        Self::Mock
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NostrProfilePublishStatus {
    Unknown,
    NotPublished,
    Publishing,
    Published,
    Failed,
}

impl Default for NostrProfilePublishStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NostrProfileEditStatus {
    Editing,
    Validating,
    AwaitingNostrAuth,
    Publishing,
    Succeeded,
    Canceled,
    Failed,
}

impl Default for NostrProfileEditStatus {
    fn default() -> Self {
        Self::Editing
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NostrProfileAction {
    Login,
    SetProfileName,
}

impl Default for NostrProfileAction {
    fn default() -> Self {
        Self::Login
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NostrAuthorizationStatus {
    Pending,
    Scanned,
    Approved,
    Rejected,
    Expired,
    Canceled,
    Failed,
}

impl Default for NostrAuthorizationStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NostrIdentity {
    pub public_key: String,
    pub npub: String,
    pub status: NostrIdentityStatus,
    pub authenticated_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NostrProfile {
    pub public_key: String,
    pub username: Option<String>,
    pub source: NostrProfileSource,
    pub publish_status: NostrProfilePublishStatus,
    pub updated_at: Option<DateTime<Utc>>,
    pub relay_urls: Vec<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NostrProfileEditRequest {
    pub draft_username: String,
    pub status: NostrProfileEditStatus,
    pub validation_error: Option<String>,
    pub identity_public_key: Option<String>,
    pub requested_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NostrAuthorizationSession {
    pub session_id: String,
    pub action: NostrProfileAction,
    pub qr_payload: String,
    pub status: NostrAuthorizationStatus,
    pub public_key: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NostrProfileError {
    EmptyUsername,
    UsernameTooLong,
    UnsafeUsername,
    SecretLikeValue,
    AuthorizationRequired,
    AuthorizationExpired,
    PublishFailed,
}

impl std::fmt::Display for NostrProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyUsername => "username is required",
            Self::UsernameTooLong => "username is too long",
            Self::UnsafeUsername => "username contains unsupported characters",
            Self::SecretLikeValue => "username looks like secret material",
            Self::AuthorizationRequired => "Nostr identity authorization is required",
            Self::AuthorizationExpired => "Nostr identity authorization expired",
            Self::PublishFailed => "Nostr profile metadata could not be saved",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NostrProfileError {}

pub fn validate_nostr_username(username: &str) -> Result<String, NostrProfileError> {
    let username = username.trim();
    if username.is_empty() {
        return Err(NostrProfileError::EmptyUsername);
    }
    if username.chars().count() > MAX_NOSTR_USERNAME_CHARS {
        return Err(NostrProfileError::UsernameTooLong);
    }
    if username.chars().any(char::is_control) {
        return Err(NostrProfileError::UnsafeUsername);
    }
    if looks_secret_like(username) {
        return Err(NostrProfileError::SecretLikeValue);
    }

    Ok(username.to_string())
}

pub fn nostr_profile_button_label(username: Option<&str>) -> String {
    format!("Set Name ({})", username.unwrap_or_default())
}

fn looks_secret_like(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "nsec", "private", "xprv", "seed", "secret", "password", "bearer", "token", "cookie",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LightningOperationKind {
    Auth,
    SendSats,
    PayInvoice,
    OpenRoute,
    CloseRoute,
    TransferAsset,
    Setup,
}

impl Default for LightningOperationKind {
    fn default() -> Self {
        Self::Setup
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LightningOperationStatus {
    Succeeded,
    ApprovalRequired,
    Pending,
    RecoverableFailure,
    Failed,
}

impl Default for LightningOperationStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LightningOperationResult {
    pub operation_id: String,
    pub operation_kind: LightningOperationKind,
    pub status: LightningOperationStatus,
    pub updated_lab_state: Option<Box<LabState>>,
    pub auth_session: Option<PlayerAuthSession>,
    pub approval: Option<TransactionApproval>,
    pub message: String,
    pub error_code: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PolarSetupStepId {
    BridgeUrl,
    ServerName,
    CreateNodes,
    GameTreasurySats,
    GameTreasuryTras,
    UserNodesSats,
    UserNodesTras,
    BlockHeight,
    UnlockRoutes,
}

impl PolarSetupStepId {
    pub fn order(self) -> u8 {
        match self {
            Self::BridgeUrl => 1,
            Self::ServerName => 2,
            Self::CreateNodes => 3,
            Self::GameTreasurySats => 4,
            Self::GameTreasuryTras => 5,
            Self::UserNodesSats => 6,
            Self::UserNodesTras => 7,
            Self::BlockHeight => 8,
            Self::UnlockRoutes => 9,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::BridgeUrl => "Bridge URLs",
            Self::ServerName => "Server Name",
            Self::CreateNodes => "Create Nodes",
            Self::GameTreasurySats => "Game Treasury (Sats)",
            Self::GameTreasuryTras => "Game Treasury (TRAs)",
            Self::UserNodesSats => "User Nodes (Sats)",
            Self::UserNodesTras => "User Nodes (TRAs)",
            Self::BlockHeight => "Block Height",
            Self::UnlockRoutes => "Unlock Routes",
        }
    }
}

impl Default for PolarSetupStepId {
    fn default() -> Self {
        Self::BridgeUrl
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PolarSetupStepStatus {
    Locked,
    Current,
    Complete,
    NeedsRetry,
    Failed,
}

impl Default for PolarSetupStepStatus {
    fn default() -> Self {
        Self::Locked
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolarSetupStep {
    pub step_id: PolarSetupStepId,
    pub order: u8,
    pub status: PolarSetupStepStatus,
    pub readiness_summary: String,
    pub last_error: Option<String>,
}

impl Default for PolarSetupStep {
    fn default() -> Self {
        let step_id = PolarSetupStepId::BridgeUrl;
        Self {
            step_id,
            order: step_id.order(),
            status: PolarSetupStepStatus::Locked,
            readiness_summary: String::new(),
            last_error: None,
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PolarConnectorHealthStatus {
    Unknown,
    Healthy,
    Unavailable,
    Unsupported,
}

impl Default for PolarConnectorHealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PolarOperationStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Retrying,
}

impl Default for PolarOperationStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolarConnectorHealth {
    pub status: PolarConnectorHealthStatus,
    pub bridge_url: String,
    pub package_name: String,
    pub message: String,
    pub checked_at: Option<DateTime<Utc>>,
}

impl Default for PolarConnectorHealth {
    fn default() -> Self {
        Self {
            status: PolarConnectorHealthStatus::Unknown,
            bridge_url: PolarAutomationProfile::default().bridge_url,
            package_name: "@lightningpolar/mcp".to_string(),
            message: "Polar connector health has not been checked.".to_string(),
            checked_at: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolarOperationRecord {
    pub operation: String,
    pub status: PolarOperationStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub attempts: u16,
    pub message: Option<String>,
}

impl PolarOperationRecord {
    pub fn pending(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            status: PolarOperationStatus::Pending,
            started_at: None,
            completed_at: None,
            attempts: 0,
            message: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolarConnectorFailure {
    pub operation: String,
    pub status: PolarConnectorHealthStatus,
    pub message: String,
    pub recovery_hint: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SetupProfile {
    pub sats_per_transaction: u64,
    pub network_name: String,
    pub setup_mode: SetupMode,
    #[serde(default)]
    pub user_auth_mode: UserAuthMode,
    #[serde(default)]
    pub player_identity: Option<PlayerIdentity>,
    #[serde(default)]
    pub last_auth_status: Option<AuthSessionStatus>,
    #[serde(default = "default_lnauth_bridge_url")]
    pub lnauth_bridge_url: String,
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
            user_auth_mode: UserAuthMode::default(),
            player_identity: None,
            last_auth_status: None,
            lnauth_bridge_url: default_lnauth_bridge_url(),
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

fn default_lnauth_bridge_url() -> String {
    DEFAULT_LNAUTH_BRIDGE_URL.to_string()
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
    #[serde(default)]
    pub local_revision: u64,
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
    #[serde(default)]
    pub player_auth_session: Option<PlayerAuthSession>,
    #[serde(default)]
    pub recent_transaction_approvals: Vec<TransactionApproval>,
    #[serde(default)]
    pub auth_warnings: Vec<String>,
    pub operation_faq: Vec<OperationFaqRow>,
    pub block_height: u64,
    #[serde(default)]
    pub polar_observed_block_height: Option<u64>,
    pub warnings: Vec<String>,
    pub action_log: Vec<ActionLogEntry>,
}

#[cfg(test)]
mod tests {
    use super::{
        nostr_profile_button_label, validate_nostr_username, NostrProfileError,
        PolarAutomationProfile, PolarConnectorHealth, PolarConnectorHealthStatus,
        PolarOperationRecord, PolarOperationStatus, SetupProfile, UserAuthMode,
        DEFAULT_NETWORK_NAME, DEFAULT_SATS_PER_TRANSACTION, MAX_NOSTR_USERNAME_CHARS,
    };

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

    #[test]
    fn setup_profile_defaults_user_auth_mode_to_app_for_old_snapshots() {
        let value = serde_json::json!({
            "sats_per_transaction": DEFAULT_SATS_PER_TRANSACTION,
            "network_name": DEFAULT_NETWORK_NAME,
            "setup_mode": "ServerConfig",
            "last_verified_at": null,
            "connection_status": "NotConfigured"
        });

        let profile: SetupProfile = serde_json::from_value(value).expect("old profile snapshot");

        assert_eq!(profile.user_auth_mode, UserAuthMode::App);
        assert_eq!(profile.player_identity, None);
        assert_eq!(profile.last_auth_status, None);
    }

    #[test]
    fn lab_state_defaults_auth_fields_for_old_snapshots() {
        let profile = SetupProfile::default();
        let state = crate::default_lab_state(profile);
        let mut value = serde_json::to_value(state).expect("lab state json");
        let object = value.as_object_mut().expect("lab state object");
        object.remove("player_auth_session");
        object.remove("recent_transaction_approvals");
        object.remove("auth_warnings");

        let state: super::LabState = serde_json::from_value(value).expect("old lab snapshot");

        assert_eq!(state.player_auth_session, None);
        assert!(state.recent_transaction_approvals.is_empty());
        assert!(state.auth_warnings.is_empty());
    }

    #[test]
    fn user_auth_mode_reports_policy_flags() {
        assert!(UserAuthMode::App.is_development_only());
        assert!(!UserAuthMode::App.requires_player_auth());
        assert!(UserAuthMode::MockLnAuth.is_mock());
        assert!(UserAuthMode::MockLnAuth.requires_player_auth());
        assert!(UserAuthMode::LnAuth.requires_authorization_event_approval());
    }

    #[test]
    fn polar_connector_health_defaults_to_unknown_local_package() {
        let health = PolarConnectorHealth::default();

        assert_eq!(health.status, PolarConnectorHealthStatus::Unknown);
        assert_eq!(health.bridge_url, "http://localhost:37373");
        assert_eq!(health.package_name, "@lightningpolar/mcp");
        assert!(health.checked_at.is_none());
    }

    #[test]
    fn polar_operation_record_pending_starts_without_attempts() {
        let record = PolarOperationRecord::pending("list_networks");

        assert_eq!(record.operation, "list_networks");
        assert_eq!(record.status, PolarOperationStatus::Pending);
        assert_eq!(record.attempts, 0);
        assert!(record.message.is_none());
    }

    #[test]
    fn nostr_username_validation_trims_valid_names() {
        assert_eq!(
            validate_nostr_username("  alice  "),
            Ok("alice".to_string())
        );
    }

    #[test]
    fn nostr_username_validation_rejects_empty_too_long_control_and_secret_like_values() {
        assert_eq!(
            validate_nostr_username("   "),
            Err(NostrProfileError::EmptyUsername)
        );
        assert_eq!(
            validate_nostr_username(&"a".repeat(MAX_NOSTR_USERNAME_CHARS + 1)),
            Err(NostrProfileError::UsernameTooLong)
        );
        assert_eq!(
            validate_nostr_username("ali\u{0007}ce"),
            Err(NostrProfileError::UnsafeUsername)
        );
        assert_eq!(
            validate_nostr_username("nsec1private"),
            Err(NostrProfileError::SecretLikeValue)
        );
    }

    #[test]
    fn nostr_profile_button_label_keeps_empty_username_shape() {
        assert_eq!(nostr_profile_button_label(None), "Set Name ()");
        assert_eq!(
            nostr_profile_button_label(Some("alice")),
            "Set Name (alice)"
        );
    }

    #[test]
    fn polar_connector_models_round_trip_json() {
        let health = PolarConnectorHealth {
            status: PolarConnectorHealthStatus::Healthy,
            bridge_url: "http://localhost:37373".to_string(),
            package_name: "@lightningpolar/mcp".to_string(),
            message: "ok".to_string(),
            checked_at: None,
        };

        let value = serde_json::to_value(&health).expect("serialize connector health");
        let parsed: PolarConnectorHealth =
            serde_json::from_value(value).expect("deserialize connector health");

        assert_eq!(parsed, health);
    }
}
