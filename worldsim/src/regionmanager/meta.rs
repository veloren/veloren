use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Server {
    connection_details: String,
}

impl Server {
    pub fn new(connection_details: String) -> Self {
        Self {
            connection_details,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RegionManagerMsg {
    Attached{server_id: u64, seed: u64},
    NewServerInMesh{server_id: u64, server_connection_details: ()},
    CreateRegion{region_id: RegionId},
    TakeOverRegionFrom{region_id: RegionId, server_id: u64},
}

pub type RegionIdSize = i8;
pub type RegionId = (/*x*/RegionIdSize, /*y*/RegionIdSize /*z = 0*/);

pub const RegionMIN:i8 = -64;
pub const RegionMAX:i8 = 63;

#[derive(Debug, Clone)]
pub struct Region {
    pub tick_time: Duration,
    pub server_id: Option<u8>,
}

impl Region {
    pub fn new(server_id: Option<u8>) -> Self {
        Self {
            tick_time: Duration::from_millis(0),
            server_id,
        }
    }
}
