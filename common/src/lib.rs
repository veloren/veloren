#![deny(unsafe_code)]
#![allow(incomplete_features)]
#![type_length_limit = "1664759"]
#![allow(clippy::option_map_unit_fn)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(
    arbitrary_enum_discriminant,
    associated_type_defaults,
    const_checked_int_methods,
    const_generics,
    fundamental,
    option_unwrap_none,
    bool_to_option,
    label_break_value,
    trait_alias,
    type_alias_impl_trait,
    option_zip
)]

pub mod assets;
pub mod astar;
pub mod character;
pub mod clock;
pub mod cmd;
pub mod combat;
pub mod comp;
pub mod consts;
pub mod effect;
pub mod event;
pub mod explosion;
pub mod figure;
pub mod generation;
pub mod grid;
pub mod loadout_builder;
pub mod lottery;
pub mod metrics;
pub mod msg;
pub mod npc;
pub mod outcome;
pub mod path;
pub mod plugin;
pub mod ray;
pub mod recipe;
pub mod region;
pub mod resources;
pub mod rtsim;
pub mod spiral;
pub mod states;
pub mod store;
pub mod sync;
pub mod terrain;
pub mod time;
pub mod typed;
pub mod util;
pub mod vol;
pub mod volumes;


pub use combat::{Damage, DamageSource, GroupTarget, Knockback};
pub use explosion::{Explosion, RadiusEffect};
pub use loadout_builder::LoadoutBuilder;
