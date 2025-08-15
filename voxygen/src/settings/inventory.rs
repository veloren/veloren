use common::comp::inventory::InventorySortOrder;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct InventorySettings {
    pub sort_order: InventorySortOrder,
}

impl Default for InventorySettings {
    fn default() -> Self {
        Self {
            sort_order: InventorySortOrder::Category,
        }
    }
}
