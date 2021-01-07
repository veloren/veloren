#![deny(unsafe_code)]
#![allow(incomplete_features)]
#![type_length_limit = "1664759"]
#![allow(clippy::option_map_unit_fn)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(
    arbitrary_enum_discriminant,
    associated_type_defaults,
    bool_to_option,
    const_checked_int_methods,
    const_generics,
    fundamental,
    iter_map_while,
    label_break_value,
    option_expect_none,
    option_unwrap_none,
    option_zip,
    trait_alias,
    type_alias_impl_trait
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
pub mod lottery;
pub mod metrics;
pub mod npc;
pub mod outcome;
pub mod path;
pub mod ray;
pub mod recipe;
pub mod region;
pub mod resources;
pub mod rtsim;
pub mod skillset_builder;
pub mod spiral;
pub mod states;
pub mod store;
pub mod terrain;
pub mod time;
pub mod typed;
pub mod uid;
pub mod util;
pub mod vol;
pub mod volumes;

pub use combat::{Damage, DamageSource, GroupTarget, Knockback};
pub use comp::inventory::loadout_builder::LoadoutBuilder;
pub use explosion::{Explosion, RadiusEffect};
pub use skillset_builder::SkillSetBuilder;
