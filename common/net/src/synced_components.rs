//! Contains an "x macro" for all synced components as well as [NetSync]
//! implementations for those components.
//!
//!
//! An x macro accepts another macro as input and calls it with a list of
//! inputs. This allows adding components to the list in the x macro declaration
//! and then writing macros that will accept this list and generate code that
//! handles every synced component without further repitition of the component
//! set.
//!
//! This module also re-exports all the component types that are synced.
//!
//! A glob import from this can be used so that the component types are in scope
//! when using the x macro defined here which requires this.

/// This provides a lowercase name and the component type.
///
/// See [module](self) level docs for more details.
#[macro_export]
macro_rules! synced_components {
    ($macro:ident) => {
        $macro! {
            body: Body,
            stats: Stats,
            buffs: Buffs,
            auras: Auras,
            energy: Energy,
            health: Health,
            poise: Poise,
            light_emitter: LightEmitter,
            loot_owner: LootOwner,
            item: Item,
            scale: Scale,
            group: Group,
            is_mount: IsMount,
            is_rider: IsRider,
            mass: Mass,
            density: Density,
            collider: Collider,
            sticky: Sticky,
            immovable: Immovable,
            character_state: CharacterState,
            shockwave: Shockwave,
            beam_segment: BeamSegment,
            alignment: Alignment,
            stance: Stance,
            // TODO: change this to `SyncFrom::ClientEntity` and sync the bare minimum
            // from other entities (e.g. just keys needed to show appearance
            // based on their loadout). Also, it looks like this actually has
            // an alternative sync method implemented in entity_sync via
            // ServerGeneral::InventoryUpdate so we could use that instead
            // or remove the part where it clones the inventory.
            inventory: Inventory,
            // TODO: this is used in combat rating calculation in voxygen but we can probably
            // remove it from that and then see if it's used for anything else and try to move
            // to only being synced for the client's entity.
            skill_set: SkillSet,

            // Synced to the client only for its own entity

            admin: Admin,
            combo: Combo,
            active_abilities: ActiveAbilities,
            can_build: CanBuild,
        }
    };
}

macro_rules! reexport_comps {
    ($($name:ident: $type:ident,)*) => {
        mod inner {
            pub use common::comp::*;
            use common::link::Is;
            use common::mounting::{Mount, Rider};

            // We alias these because the identifier used for the
            // component's type is reused as an enum variant name
            // in the macro's that we pass to `synced_components!`.
            //
            // This is also the reason we need this inner module, since
            // we can't just re-export all the types directly from `common::comp`.
            pub type IsMount = Is<Mount>;
            pub type IsRider = Is<Rider>;
        }

        // Re-export all the component types. So that uses of `synced_components!` outside this
        // module can bring them into scope with a single glob import.
        $(pub use inner::$type;)*
    }
}
// Pass `reexport_comps` macro to the "x macro" which will invoke it with a list
// of components.
//
// Note: this brings all these components into scope for the implementations
// below.
synced_components!(reexport_comps);

// ===============================
// === NetSync implementations ===
// ===============================

use crate::sync::{NetSync, SyncFrom};

// These are synced from any entity within range.

impl NetSync for Body {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Stats {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Buffs {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Auras {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Energy {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Health {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;

    fn pre_insert(&mut self, world: &specs::World) {
        use common::resources::Time;
        use specs::WorldExt;

        // Time isn't synced between client and server so replace the Time from the
        // server with the Client's local Time to enable accurate comparison.
        self.last_change.time = *world.read_resource::<Time>();
    }

    fn pre_modify(&mut self, world: &specs::World) {
        use common::resources::Time;
        use specs::WorldExt;

        // Time isn't synced between client and server so replace the Time from the
        // server with the Client's local Time to enable accurate comparison.
        self.last_change.time = *world.read_resource::<Time>();
    }
}

impl NetSync for Poise {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for LightEmitter {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for LootOwner {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Item {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Scale {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Group {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for IsMount {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for IsRider {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Mass {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Density {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Collider {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Sticky {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Immovable {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for CharacterState {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Shockwave {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for BeamSegment {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Alignment {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Inventory {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for SkillSet {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

impl NetSync for Stance {
    const SYNC_FROM: SyncFrom = SyncFrom::AnyEntity;
}

// These are synced only from the client's own entity.

impl NetSync for Admin {
    const SYNC_FROM: SyncFrom = SyncFrom::ClientEntity;
}

impl NetSync for Combo {
    const SYNC_FROM: SyncFrom = SyncFrom::ClientEntity;
}

impl NetSync for ActiveAbilities {
    const SYNC_FROM: SyncFrom = SyncFrom::ClientEntity;
}

impl NetSync for CanBuild {
    const SYNC_FROM: SyncFrom = SyncFrom::ClientEntity;
}
