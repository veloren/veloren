// Note: Currently only one-way sync is supported until a usecase for two-way
// sync arises
pub mod interpolation;
mod net_sync;
mod packet;
mod sync_ext;
mod track;

// Reexports
pub use common::uid::{Uid, UidAllocator};
pub use net_sync::{NetSync, SyncFrom};
pub use packet::{
    handle_insert, handle_interp_insert, handle_interp_modify, handle_interp_remove, handle_modify,
    handle_remove, CompPacket, CompSyncPackage, EntityPackage, EntitySyncPackage,
    InterpolatableComponent,
};
pub use sync_ext::WorldSyncExt;
pub use track::UpdateTracker;
