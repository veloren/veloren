#![deny(unsafe_code)]
#![type_length_limit = "1664759"]
#![expect(clippy::option_map_unit_fn)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(
    fundamental,
    trait_alias,
    type_changing_struct_update,
    macro_metavar_expr
)]

pub use common_assets as assets;
pub use uuid;

// Modules

pub mod combat;
pub mod comp;
pub mod consts;
pub mod resources;
pub mod shared_server_config;
pub mod uid;

pub mod astar;
pub mod calendar;
pub mod character;
pub mod clock;
pub mod cmd;
pub mod depot;
pub mod effect;
pub mod event;
pub mod explosion;
pub mod figure;
pub mod generation;
pub mod grid;
pub mod interaction;
pub mod link;
pub mod lod;
pub mod lottery;
pub mod mounting;
pub mod npc;
pub mod outcome;
pub mod path;
pub mod ray;
pub mod recipe;
pub mod region;
pub mod rtsim;
pub mod skillset_builder;
pub mod slowjob;
pub mod spiral;
pub mod spot;
pub mod states;
pub mod store;
pub mod terrain;
pub mod tether;
pub mod time;
pub mod trade;
pub mod util;
pub mod vol;
pub mod volumes;
pub mod weather;

mod cached_spatial_grid;
mod view_distances;

// We declare a macro in this module so there are issues referring to it by path
// within this crate if typed module is declared in macro expansion.
pub mod typed;

pub use combat::{DamageKind, DamageSource};

pub use cached_spatial_grid::CachedSpatialGrid;
pub use combat::{Damage, GroupTarget, Knockback, KnockbackDir};
pub use comp::inventory::loadout_builder::LoadoutBuilder;
pub use explosion::{Explosion, RadiusEffect};
pub use skillset_builder::SkillSetBuilder;
pub use view_distances::ViewDistances;
