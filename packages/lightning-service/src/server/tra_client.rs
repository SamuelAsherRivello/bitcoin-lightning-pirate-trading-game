use crate::client::error::LightningError;
use crate::client::models::{
    DemoNodeId, MintTraRequest, TraItem, TraOwnershipStatus, TraTransferStatus, TransferTraRequest,
    APPLE_ITEM_ID, BOOK_ITEM_ID,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraCapabilityStatus {
    Available,
    MissingAsset,
    UnsupportedMetadata,
    ProofPending,
    TransferFailed,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraClientConfig {
    pub endpoint: String,
}

impl TraClientConfig {
    pub fn is_configured(&self) -> bool {
        !self.endpoint.trim().is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaprootAssetsClient {
    config: TraClientConfig,
}

impl TaprootAssetsClient {
    pub fn new(config: TraClientConfig) -> Self {
        Self { config }
    }

    pub fn verify_capability(&self) -> Result<TraCapabilityStatus, LightningError> {
        if self.config.is_configured() {
            Ok(TraCapabilityStatus::Available)
        } else {
            Ok(TraCapabilityStatus::Unavailable)
        }
    }

    pub fn mint_or_discover(
        &self,
        request: &MintTraRequest,
        asset_index: usize,
    ) -> Result<TraItem, LightningError> {
        if self.verify_capability()? != TraCapabilityStatus::Available {
            return Err(LightningError::TraAdapterUnavailable);
        }

        Ok(TraItem {
            tra_id: format!("tra-{}", asset_index + 1),
            asset_id: format!("regtest-asset-{}", asset_index + 1),
            unique_name: request.unique_name.clone(),
            item_id: request.item_id,
            owner_node: request.owner_node,
            ownership_status: recovery_status_for_item_id(request.item_id),
            transfer_status: TraTransferStatus::None,
        })
    }

    pub fn transfer(
        &self,
        item: &TraItem,
        request: &TransferTraRequest,
    ) -> Result<TraItem, LightningError> {
        if self.verify_capability()? != TraCapabilityStatus::Available {
            return Err(LightningError::TraAdapterUnavailable);
        }
        if item.owner_node != request.from_node {
            return Err(LightningError::TraOwnerMismatch);
        }

        let mut next_item = item.clone();
        next_item.owner_node = request.to_node;
        next_item.ownership_status = TraOwnershipStatus::Verified;
        next_item.transfer_status = TraTransferStatus::Succeeded;
        Ok(next_item)
    }

    pub fn verify_owner(
        &self,
        item: &TraItem,
        expected_owner: DemoNodeId,
    ) -> Result<TraOwnershipStatus, LightningError> {
        if self.verify_capability()? != TraCapabilityStatus::Available {
            return Err(LightningError::TraAdapterUnavailable);
        }
        if item.owner_node == expected_owner {
            Ok(item.ownership_status)
        } else {
            Ok(TraOwnershipStatus::Failed)
        }
    }

    pub fn map_recovery_status(status: TraCapabilityStatus) -> TraOwnershipStatus {
        match status {
            TraCapabilityStatus::Available => TraOwnershipStatus::Verified,
            TraCapabilityStatus::MissingAsset => TraOwnershipStatus::Missing,
            TraCapabilityStatus::UnsupportedMetadata => TraOwnershipStatus::Unsupported,
            TraCapabilityStatus::ProofPending => TraOwnershipStatus::Pending,
            TraCapabilityStatus::TransferFailed | TraCapabilityStatus::Unavailable => {
                TraOwnershipStatus::Failed
            }
        }
    }
}

fn recovery_status_for_item_id(item_id: u32) -> TraOwnershipStatus {
    if item_id == BOOK_ITEM_ID || item_id == APPLE_ITEM_ID {
        TraOwnershipStatus::Verified
    } else {
        TraOwnershipStatus::Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> TaprootAssetsClient {
        TaprootAssetsClient::new(TraClientConfig {
            endpoint: "http://localhost:10029".to_string(),
        })
    }

    #[test]
    fn capability_requires_configured_endpoint() {
        let unavailable = TaprootAssetsClient::new(TraClientConfig {
            endpoint: String::new(),
        });

        assert_eq!(
            unavailable.verify_capability().expect("capability status"),
            TraCapabilityStatus::Unavailable
        );
        assert_eq!(
            client().verify_capability().expect("capability status"),
            TraCapabilityStatus::Available
        );
    }

    #[test]
    fn mint_and_transfer_fake_adapter_item() {
        let request = MintTraRequest {
            owner_node: DemoNodeId::Bob,
            unique_name: "Book".to_string(),
            item_id: BOOK_ITEM_ID,
        };
        let item = client()
            .mint_or_discover(&request, 0)
            .expect("mint fake TRA item");
        let item = client()
            .transfer(
                &item,
                &TransferTraRequest {
                    tra_id: item.tra_id.clone(),
                    from_node: DemoNodeId::Bob,
                    to_node: DemoNodeId::Alice,
                },
            )
            .expect("transfer fake TRA item");

        assert_eq!(item.owner_node, DemoNodeId::Alice);
        assert_eq!(item.transfer_status, TraTransferStatus::Succeeded);
    }

    #[test]
    fn recovery_mapping_covers_adapter_states() {
        assert_eq!(
            TaprootAssetsClient::map_recovery_status(TraCapabilityStatus::MissingAsset),
            TraOwnershipStatus::Missing
        );
        assert_eq!(
            TaprootAssetsClient::map_recovery_status(TraCapabilityStatus::UnsupportedMetadata),
            TraOwnershipStatus::Unsupported
        );
        assert_eq!(
            TaprootAssetsClient::map_recovery_status(TraCapabilityStatus::ProofPending),
            TraOwnershipStatus::Pending
        );
        assert_eq!(
            TaprootAssetsClient::map_recovery_status(TraCapabilityStatus::TransferFailed),
            TraOwnershipStatus::Failed
        );
    }
}
