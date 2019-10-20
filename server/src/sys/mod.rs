pub mod entity_sync;
pub mod message;
pub mod subscription;
pub mod terrain;
pub mod terrain_sync;

use specs::DispatcherBuilder;

// System names
const ENTITY_SYNC_SYS: &str = "server_entity_sync_sys";
const SUBSCRIPTION_SYS: &str = "server_subscription_sys";
const TERRAIN_SYNC_SYS: &str = "server_terrain_sync_sys";
const TERRAIN_SYS: &str = "server_terrain_sys";
const MESSAGE_SYS: &str = "server_message_sys";
//const SYNC_CHUNK_SYS: &str = "server_sync_chunk_sys";

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(subscription::Sys, SUBSCRIPTION_SYS, &[]);
    dispatch_builder.add(entity_sync::Sys, ENTITY_SYNC_SYS, &[SUBSCRIPTION_SYS]);
    dispatch_builder.add(terrain_sync::Sys, TERRAIN_SYS, &[]);
    dispatch_builder.add(terrain::Sys, TERRAIN_SYNC_SYS, &[TERRAIN_SYS]);
    dispatch_builder.add(message::Sys, MESSAGE_SYS, &[]);
}
