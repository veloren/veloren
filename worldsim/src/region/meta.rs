use crate::regionmanager::meta::RegionId;

#[derive(Debug, Clone)]
pub struct RegionMeta {
    id: RegionId,
}

impl RegionMeta {
    pub fn new(id: RegionId) -> Self {
        Self {
            id,
        }
    }
}