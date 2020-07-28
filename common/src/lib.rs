#![deny(unsafe_code)]
#![allow(clippy::option_map_unit_fn)]
#![type_length_limit = "1664759"]
#![feature(
    arbitrary_enum_discriminant,
    const_checked_int_methods,
    const_if_match,
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
pub mod comp;
pub mod effect;
pub mod event;
pub mod figure;
pub mod generation;
pub mod loadout_builder;
pub mod msg;
pub mod npc;
pub mod path;
pub mod ray;
pub mod recipe;
pub mod region;
pub mod spiral;
pub mod state;
pub mod states;
pub mod store;
pub mod sync;
pub mod sys;
pub mod terrain;
pub mod util;
pub mod vol;
pub mod volumes;

pub use loadout_builder::LoadoutBuilder;
