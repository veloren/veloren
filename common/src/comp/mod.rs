#[cfg(not(target_arch = "wasm32"))] mod ability;
#[cfg(not(target_arch = "wasm32"))] mod admin;
#[cfg(not(target_arch = "wasm32"))] pub mod agent;
#[cfg(not(target_arch = "wasm32"))] pub mod aura;
#[cfg(not(target_arch = "wasm32"))] pub mod beam;
#[cfg(not(target_arch = "wasm32"))] pub mod body;
pub mod buff;
#[cfg(not(target_arch = "wasm32"))]
mod character_state;
#[cfg(not(target_arch = "wasm32"))] pub mod chat;
#[cfg(not(target_arch = "wasm32"))] pub mod combo;
#[cfg(not(target_arch = "wasm32"))]
mod controller;
#[cfg(not(target_arch = "wasm32"))] mod energy;
#[cfg(not(target_arch = "wasm32"))] pub mod group;
mod health;
#[cfg(not(target_arch = "wasm32"))]
pub mod home_chunk;
#[cfg(not(target_arch = "wasm32"))] mod inputs;
#[cfg(not(target_arch = "wasm32"))]
pub mod inventory;
#[cfg(not(target_arch = "wasm32"))]
pub mod invite;
#[cfg(not(target_arch = "wasm32"))] mod last;
#[cfg(not(target_arch = "wasm32"))] mod location;
#[cfg(not(target_arch = "wasm32"))] mod misc;
#[cfg(not(target_arch = "wasm32"))] pub mod ori;
#[cfg(not(target_arch = "wasm32"))] mod phys;
#[cfg(not(target_arch = "wasm32"))] mod player;
#[cfg(not(target_arch = "wasm32"))] pub mod poise;
#[cfg(not(target_arch = "wasm32"))]
pub mod projectile;
#[cfg(not(target_arch = "wasm32"))]
pub mod shockwave;
#[cfg(not(target_arch = "wasm32"))]
pub mod skills;
#[cfg(not(target_arch = "wasm32"))] mod stats;
#[cfg(not(target_arch = "wasm32"))]
pub mod visual;

// Reexports
#[cfg(not(target_arch = "wasm32"))]
pub use self::{
    ability::{CharacterAbility, CharacterAbilityType},
    admin::Admin,
    agent::{Agent, Alignment},
    aura::{Aura, AuraChange, AuraKind, Auras},
    beam::{Beam, BeamSegment},
    body::{
        biped_large, biped_small, bird_medium, bird_small, dragon, fish_medium, fish_small, golem,
        humanoid, object, quadruped_low, quadruped_medium, quadruped_small, ship, theropod,
        AllBodies, Body, BodyData,
    },
    buff::{
        Buff, BuffCategory, BuffChange, BuffData, BuffEffect, BuffId, BuffKind, BuffSource, Buffs,
        ModifierKind,
    },
    character_state::{CharacterState, InputAttr, Melee, StateUpdate},
    chat::{
        ChatMode, ChatMsg, ChatType, Faction, SpeechBubble, SpeechBubbleType, UnresolvedChatMsg,
    },
    combo::Combo,
    controller::{
        Climb, ControlAction, ControlEvent, Controller, ControllerInputs, GroupManip, InputKind,
        InventoryAction, InventoryEvent, InventoryManip, MountState, Mounting,
    },
    energy::{Energy, EnergyChange, EnergySource},
    group::Group,
    home_chunk::HomeChunk,
    inputs::CanBuild,
    inventory::{
        item::{self, tool, Item, ItemConfig, ItemDrop},
        slot, Inventory, InventoryUpdate, InventoryUpdateEvent,
    },
    last::Last,
    location::{Waypoint, WaypointArea},
    misc::Object,
    ori::Ori,
    phys::{
        Collider, ForceUpdate, Gravity, Mass, PhysicsState, Pos, PosVelDefer, PreviousPhysCache,
        Scale, Sticky, Vel,
    },
    player::Player,
    poise::{Poise, PoiseChange, PoiseSource, PoiseState},
    projectile::{Projectile, ProjectileConstructor},
    shockwave::{Shockwave, ShockwaveHitEntities},
    skills::{Skill, SkillGroup, SkillGroupKind, SkillSet},
    stats::Stats,
    visual::{LightAnimation, LightEmitter},
};

pub use health::{Health, HealthChange, HealthSource};
