use thiserror::Error;

#[derive(Debug, Error)]
pub enum LightningError {
    #[error("Sats per transaction must be a whole number from 1 to 100,000.")]
    InvalidDemoAmount,

    #[error("The local server profile is missing Alice, Bob, or Carol.")]
    MissingRequiredNodes,

    #[error("Paste Polar endpoint, TLS cert, and macaroon values for Alice, Bob, and Carol.")]
    MissingPolarConnectionValues,

    #[error("Paste the local Polar bridge URL before creating Lightning nodes.")]
    MissingPolarAutomationValues,

    #[error("The app rejected a non-regtest profile. Only local Polar regtest profiles are supported in this POC.")]
    NonRegtestProfile,

    #[error("The app rejected a hosted or non-local endpoint. Only local lab endpoints are supported in this POC.")]
    NonLocalEndpoint,

    #[error("The Polar automation bridge must be a local localhost or 127.0.0.1 URL.")]
    NonLocalPolarBridge,

    #[error("Setup must be connected before this action can run.")]
    SetupIncomplete,

    #[error("The selected trade is already active, closing, or under construction.")]
    RouteAlreadyExists,

    #[error("The selected trade is not active yet. Use Wait for Next Block first.")]
    RouteNotActive,

    #[error("The selected trade is already closed or closing.")]
    RouteAlreadyClosing,

    #[error(
        "The selected trade route does not have enough outbound liquidity for this demo payment."
    )]
    InsufficientLiquidity,

    #[error("The selected invoice is missing or cannot be paid.")]
    InvoiceUnavailable,

    #[error("The selected TRA item is missing or cannot be transferred.")]
    TraItemUnavailable,

    #[error("The selected node already has the maximum of 3 TRA inventory items.")]
    TraInventoryFull,

    #[error("The selected TRA item type is not supported by the game catalog.")]
    UnsupportedTraItemType,

    #[error("TRA item names must be unique in the local lab.")]
    DuplicateTraItemName,

    #[error("The selected TRA item is owned by a different node.")]
    TraOwnerMismatch,

    #[error("The local Taproot Assets adapter is unavailable.")]
    TraAdapterUnavailable,

    #[error("Create and fund the Game Treasury before continuing setup.")]
    GameTreasuryNotReady,

    #[error("The Game Treasury does not have enough sats for this game action.")]
    GameTreasuryInsufficientSats,

    #[error("The Game Treasury is missing an item required for NPC setup.")]
    GameTreasuryItemUnavailable,

    #[error(
        "Treasury history cannot include wallet secrets, macaroons, seeds, or proof material."
    )]
    SensitiveTreasuryDetail,

    #[error("Cannot load the local server profile: {0}")]
    ConfigLoadFailed(String),

    #[error("Cannot save or load local lab state: {0}")]
    StorageFailed(String),
}
