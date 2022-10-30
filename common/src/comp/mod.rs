#[cfg(not(target_arch = "wasm32"))]
pub mod ability;
#[cfg(not(target_arch = "wasm32"))] mod admin;
#[cfg(not(target_arch = "wasm32"))] pub mod agent;
#[cfg(not(target_arch = "wasm32"))]
pub mod anchor;
#[cfg(not(target_arch = "wasm32"))] pub mod aura;
#[cfg(not(target_arch = "wasm32"))] pub mod beam;
#[cfg(not(target_arch = "wasm32"))] pub mod body;
pub mod buff;
#[cfg(not(target_arch = "wasm32"))]
pub mod character_state;
#[cfg(not(target_arch = "wasm32"))] pub mod chat;
#[cfg(not(target_arch = "wasm32"))] pub mod combo;
pub mod compass;
#[cfg(not(target_arch = "wasm32"))]
pub mod controller;
#[cfg(not(target_arch = "wasm32"))]
pub mod dialogue;
#[cfg(not(target_arch = "wasm32"))] mod energy;
#[cfg(not(target_arch = "wasm32"))]
pub mod fluid_dynamics;
#[cfg(not(target_arch = "wasm32"))] pub mod group;
mod health;
#[cfg(not(target_arch = "wasm32"))] mod inputs;
#[cfg(not(target_arch = "wasm32"))]
pub mod inventory;
#[cfg(not(target_arch = "wasm32"))]
pub mod invite;
#[cfg(not(target_arch = "wasm32"))] mod last;
#[cfg(not(target_arch = "wasm32"))] mod location;
pub mod loot_owner;
#[cfg(not(target_arch = "wasm32"))] pub mod melee;
#[cfg(not(target_arch = "wasm32"))] mod misc;
#[cfg(not(target_arch = "wasm32"))] pub mod ori;
#[cfg(not(target_arch = "wasm32"))] pub mod pet;
#[cfg(not(target_arch = "wasm32"))] mod phys;
#[cfg(not(target_arch = "wasm32"))] mod player;
#[cfg(not(target_arch = "wasm32"))] pub mod poise;
#[cfg(not(target_arch = "wasm32"))]
pub mod projectile;
#[cfg(not(target_arch = "wasm32"))]
pub mod shockwave;
#[cfg(not(target_arch = "wasm32"))]
pub mod skillset;
#[cfg(not(target_arch = "wasm32"))] mod stats;
#[cfg(not(target_arch = "wasm32"))]
pub mod visual;

// Reexports
#[cfg(not(target_arch = "wasm32"))]
pub use self::{
    ability::{
        Ability, AbilityInput, ActiveAbilities, CharacterAbility, CharacterAbilityType, Stance,
        MAX_ABILITIES,
    },
    admin::{Admin, AdminRole},
    agent::{
        Agent, Alignment, Behavior, BehaviorCapability, BehaviorState, PidController,
        TradingBehavior,
    },
    anchor::Anchor,
    aura::{Aura, AuraChange, AuraKind, Auras},
    beam::{Beam, BeamSegment},
    body::{
        arthropod, biped_large, biped_small, bird_large, bird_medium, dragon, fish_medium,
        fish_small, golem, humanoid, item_drop, object, quadruped_low, quadruped_medium,
        quadruped_small, ship, theropod, AllBodies, Body, BodyData,
    },
    buff::{
        Buff, BuffCategory, BuffChange, BuffData, BuffEffect, BuffId, BuffKind, BuffSource, Buffs,
        ModifierKind,
    },
    character_state::{CharacterState, StateUpdate},
    chat::{
        ChatMode, ChatMsg, ChatType, Faction, SpeechBubble, SpeechBubbleType, UnresolvedChatMsg,
    },
    combo::Combo,
    controller::{
        Climb, ControlAction, ControlEvent, Controller, ControllerInputs, GroupManip, InputAttr,
        InputKind, InventoryAction, InventoryEvent, InventoryManip, UtteranceKind,
    },
    energy::Energy,
    fluid_dynamics::Fluid,
    group::Group,
    inputs::CanBuild,
    inventory::{
        item::{
            self,
            item_key::ItemKey,
            tool::{self, AbilityItem},
            Item, ItemConfig, ItemDrop,
        },
        slot, CollectFailedReason, Inventory, InventoryUpdate, InventoryUpdateEvent,
    },
    last::Last,
    location::{MapMarker, MapMarkerChange, MapMarkerUpdate, Waypoint, WaypointArea},
    loot_owner::LootOwner,
    melee::{Melee, MeleeConstructor, MeleeConstructorKind},
    misc::Object,
    ori::Ori,
    pet::Pet,
    phys::{
        Collider, Density, ForceUpdate, Immovable, Mass, PhysicsState, Pos, PosVelOriDefer,
        PreviousPhysCache, Scale, Sticky, Vel,
    },
    player::DisconnectReason,
    player::{AliasError, Player, MAX_ALIAS_LEN},
    poise::{Poise, PoiseChange, PoiseState},
    projectile::{Projectile, ProjectileConstructor},
    shockwave::{Shockwave, ShockwaveHitEntities},
    skillset::{
        skills::{self, Skill},
        SkillGroup, SkillGroupKind, SkillSet,
    },
    stats::{Stats, StatsModifier},
    visual::{LightAnimation, LightEmitter},
};

pub use health::{Health, HealthChange};
