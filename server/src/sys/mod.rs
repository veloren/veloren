pub mod sync;
//pub mod sync_chunk;
pub mod message;
pub mod subscription;

use specs::DispatcherBuilder;

// System names
const SYNC_SYS: &str = "server_sync_sys";
const SUBSCRIPTION_SYS: &str = "server_subscription_sys";
const MESSAGE_SYS: &str = "server_message_sys";
//const SYNC_CHUNK_SYS: &str = "server_sync_chunk_sys";

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(subscription::Sys, SUBSCRIPTION_SYS, &[]);
    dispatch_builder.add(sync::Sys, SYNC_SYS, &[SUBSCRIPTION_SYS]);
    dispatch_builder.add(message::Sys, MESSAGE_SYS, &[]);
    //dispatch_builder.add(sync_chunk::Sys, SYNC_CHUNKR_SYS, &[]);
}
