use chrono::{DateTime, Utc};
pub use lightning_service::{
    ActionLogEntry, BlockWaitAction, BlockWaitReason, BlockWaitStatus, ConnectionStatus, DemoNode,
    DemoNodeId, GameItemDefinition, GameTreasury, InvoiceRequest, InvoiceStatus, LabState,
    Location, MintTraRequest, NodeRole, NodeStatus, NpcItemTransfer, OperationFaqRow,
    PaymentAttempt, PaymentStatus, PolarAutomationProfile, PolarConnectionProfile,
    PolarNodeConnection, RouteStatus, SetupMode, SetupProfile, TraItem, TraOwnershipStatus,
    TraTransferStatus, TradeRoute, TransferTraRequest, TreasuryEntry, TreasuryEntryDirection,
    TreasuryImpactPreview, TreasuryResource, TreasuryStatus, APPLE_ITEM_ID, BOOK_ITEM_ID,
    DEFAULT_BITCOIN_BACKEND_NAME, DEFAULT_NETWORK_NAME, DEFAULT_ROUTE_CAPACITY_SATS,
    DEFAULT_SATS_PER_TRANSACTION, GAME_TREASURY_NODE_LABEL, MAX_SATS_PER_TRANSACTION,
    MAX_TRA_ITEMS_PER_NODE,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TemplateData {
    pub id: i64,
    pub message: String,
}

impl TemplateData {
    pub fn seed() -> Self {
        Self {
            id: 1,
            message: "Hello, World!".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum TemplateDataSource {
    BrowserSnapshot,
    Database,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TemplateDataLoadResult {
    pub data: TemplateData,
    pub source: TemplateDataSource,
    pub db_last_loaded_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemplateDataLoadRequest {
    pub sequence: u64,
}

impl TemplateDataLoadRequest {
    pub fn initial() -> Self {
        Self { sequence: 0 }
    }
}
