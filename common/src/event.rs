use crate::{
    Explosion,
    character::CharacterId,
    combat::{AttackSource, DeathEffects},
    comp::{
        self, DisconnectReason, LootOwner, Ori, Pos, UnresolvedChatMsg, Vel,
        agent::Sound,
        dialogue::Subject,
        invite::{InviteKind, InviteResponse},
    },
    generation::{EntityInfo, SpecialEntity},
    interaction::Interaction,
    lottery::LootSpec,
    mounting::VolumePos,
    outcome::Outcome,
    resources::Secs,
    rtsim::{self, RtSimEntity},
    terrain::SpriteKind,
    trade::{TradeAction, TradeId},
    uid::Uid,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use specs::Entity as EcsEntity;
use std::{collections::VecDeque, sync::Mutex};
use uuid::Uuid;
use vek::*;

pub type SiteId = u64;
/// Plugin identifier (sha256)
pub type PluginHash = [u8; 32];

pub enum LocalEvent {
    /// Applies upward force to entity's `Vel`
    Jump(EcsEntity, f32),
    /// Applies the `impulse` to `entity`'s `Vel`
    ApplyImpulse {
        entity: EcsEntity,
        impulse: Vec3<f32>,
    },
    /// Applies `vel` velocity to `entity`
    Boost { entity: EcsEntity, vel: Vec3<f32> },
    /// Creates an outcome
    CreateOutcome(Outcome),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UpdateCharacterMetadata {
    pub skill_set_persistence_load_error: Option<comp::skillset::SkillsPersistenceError>,
}

pub struct NpcBuilder {
    pub stats: comp::Stats,
    pub skill_set: comp::SkillSet,
    pub health: Option<comp::Health>,
    pub poise: comp::Poise,
    pub inventory: comp::inventory::Inventory,
    pub body: comp::Body,
    pub agent: Option<comp::Agent>,
    pub alignment: comp::Alignment,
    pub scale: comp::Scale,
    pub anchor: Option<comp::Anchor>,
    pub loot: LootSpec<String>,
    pub pets: Vec<(NpcBuilder, Vec3<f32>)>,
    pub rtsim_entity: Option<RtSimEntity>,
    pub projectile: Option<comp::Projectile>,
    pub heads: Option<comp::body::parts::Heads>,
    pub death_effects: Option<DeathEffects>,
}

impl NpcBuilder {
    pub fn new(stats: comp::Stats, body: comp::Body, alignment: comp::Alignment) -> Self {
        Self {
            stats,
            skill_set: comp::SkillSet::default(),
            health: None,
            poise: comp::Poise::new(body),
            inventory: comp::Inventory::with_empty(),
            body,
            agent: None,
            alignment,
            scale: comp::Scale(1.0),
            anchor: None,
            loot: LootSpec::Nothing,
            rtsim_entity: None,
            projectile: None,
            pets: Vec::new(),
            heads: None,
            death_effects: None,
        }
    }

    pub fn with_heads(mut self, heads: impl Into<Option<comp::body::parts::Heads>>) -> Self {
        self.heads = heads.into();
        self
    }

    pub fn with_health(mut self, health: impl Into<Option<comp::Health>>) -> Self {
        self.health = health.into();
        self
    }

    pub fn with_poise(mut self, poise: comp::Poise) -> Self {
        self.poise = poise;
        self
    }

    pub fn with_agent(mut self, agent: impl Into<Option<comp::Agent>>) -> Self {
        self.agent = agent.into();
        self
    }

    pub fn with_anchor(mut self, anchor: comp::Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    pub fn with_rtsim(mut self, rtsim: RtSimEntity) -> Self {
        self.rtsim_entity = Some(rtsim);
        self
    }

    pub fn with_projectile(mut self, projectile: impl Into<Option<comp::Projectile>>) -> Self {
        self.projectile = projectile.into();
        self
    }

    pub fn with_scale(mut self, scale: comp::Scale) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_inventory(mut self, inventory: comp::Inventory) -> Self {
        self.inventory = inventory;
        self
    }

    pub fn with_skill_set(mut self, skill_set: comp::SkillSet) -> Self {
        self.skill_set = skill_set;
        self
    }

    pub fn with_loot(mut self, loot: LootSpec<String>) -> Self {
        self.loot = loot;
        self
    }

    pub fn with_pets(mut self, pets: Vec<(NpcBuilder, Vec3<f32>)>) -> Self {
        self.pets = pets;
        self
    }

    pub fn with_death_effects(mut self, death_effects: Option<DeathEffects>) -> Self {
        self.death_effects = death_effects;
        self
    }
}

// These events are generated only by server systems
//
// TODO: we may want to move these into the server crate, this may allow moving
// other types out of `common` and would also narrow down where we know specific
// events will be emitted (if done it should probably be setup so they can
// easily be moved back here if needed).

pub struct ClientDisconnectEvent(pub EcsEntity, pub DisconnectReason);

pub struct ClientDisconnectWithoutPersistenceEvent(pub EcsEntity);

pub struct CommandEvent(pub EcsEntity, pub String, pub Vec<String>);

pub struct CreateSpecialEntityEvent {
    pub pos: Vec3<f32>,
    pub entity: SpecialEntity,
}

pub struct CreateShipEvent {
    pub pos: Pos,
    pub ori: Ori,
    pub ship: comp::ship::Body,
    pub rtsim_entity: Option<RtSimEntity>,
    pub driver: Option<NpcBuilder>,
}

pub struct CreateItemDropEvent {
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
    pub item: comp::PickupItem,
    pub loot_owner: Option<LootOwner>,
}
pub struct CreateObjectEvent {
    pub pos: Pos,
    pub vel: Vel,
    pub body: comp::object::Body,
    pub object: Option<comp::Object>,
    pub item: Option<comp::PickupItem>,
    pub light_emitter: Option<comp::LightEmitter>,
    pub stats: Option<comp::Stats>,
}

/// Inserts default components for a character when loading into the game.
pub struct InitializeCharacterEvent {
    pub entity: EcsEntity,
    pub character_id: CharacterId,
    pub requested_view_distances: crate::ViewDistances,
}

pub struct InitializeSpectatorEvent(pub EcsEntity, pub crate::ViewDistances);

pub struct UpdateCharacterDataEvent {
    pub entity: EcsEntity,
    pub components: (
        comp::Body,
        Option<comp::Hardcore>,
        comp::Stats,
        comp::SkillSet,
        comp::Inventory,
        Option<comp::Waypoint>,
        Vec<(comp::Pet, comp::Body, comp::Stats)>,
        comp::ActiveAbilities,
        Option<comp::MapMarker>,
    ),
    pub metadata: UpdateCharacterMetadata,
}

pub struct ExitIngameEvent {
    pub entity: EcsEntity,
}

pub struct RequestSiteInfoEvent {
    pub entity: EcsEntity,
    pub id: SiteId,
}

pub struct TamePetEvent {
    pub pet_entity: EcsEntity,
    pub owner_entity: EcsEntity,
}

pub struct UpdateMapMarkerEvent {
    pub entity: EcsEntity,
    pub update: comp::MapMarkerChange,
}

pub struct MakeAdminEvent {
    pub entity: EcsEntity,
    pub admin: comp::Admin,
    pub uuid: Uuid,
}

pub struct DeleteCharacterEvent {
    pub entity: EcsEntity,
    pub requesting_player_uuid: String,
    pub character_id: CharacterId,
}

pub struct TeleportToPositionEvent {
    pub entity: EcsEntity,
    pub position: Vec3<f32>,
}

pub struct RequestPluginsEvent {
    pub entity: EcsEntity,
    pub plugins: Vec<PluginHash>,
}

// These events are generated in common systems in addition to server systems
// (but note on the client the event buses aren't registered and these events
// aren't actually emitted).

pub struct ChatEvent(pub UnresolvedChatMsg);

pub struct CreateNpcEvent {
    pub pos: Pos,
    pub ori: Ori,
    pub npc: NpcBuilder,
    pub rider: Option<NpcBuilder>,
}

pub struct CreateAuraEntityEvent {
    pub auras: comp::Auras,
    pub pos: Pos,
    pub creator_uid: Uid,
    pub duration: Option<Secs>,
}

pub struct ExplosionEvent {
    pub pos: Vec3<f32>,
    pub explosion: Explosion,
    pub owner: Option<Uid>,
}

pub struct BonkEvent {
    pub pos: Vec3<f32>,
    pub owner: Option<Uid>,
    pub target: Option<Uid>,
}

pub struct HealthChangeEvent {
    pub entity: EcsEntity,
    pub change: comp::HealthChange,
}

pub struct KillEvent {
    pub entity: EcsEntity,
}

pub struct HelpDownedEvent {
    pub helper: Option<Uid>,
    pub target: Uid,
}

pub struct DownedEvent {
    pub entity: EcsEntity,
}

pub struct PoiseChangeEvent {
    pub entity: EcsEntity,
    pub change: comp::PoiseChange,
}

pub struct DeleteEvent(pub EcsEntity);

pub struct DestroyEvent {
    pub entity: EcsEntity,
    pub cause: comp::HealthChange,
}

pub struct InventoryManipEvent(pub EcsEntity, pub comp::InventoryManip);

pub struct GroupManipEvent(pub EcsEntity, pub comp::GroupManip);

pub struct RespawnEvent(pub EcsEntity);

pub struct ShootEvent {
    pub entity: EcsEntity,
    pub pos: Pos,
    pub dir: Dir,
    pub body: comp::Body,
    pub light: Option<comp::LightEmitter>,
    pub projectile: comp::Projectile,
    pub speed: f32,
    pub object: Option<comp::Object>,
}

pub struct ShockwaveEvent {
    pub properties: comp::shockwave::Properties,
    pub pos: Pos,
    pub ori: Ori,
}

pub struct KnockbackEvent {
    pub entity: EcsEntity,
    pub impulse: Vec3<f32>,
}

pub struct LandOnGroundEvent {
    pub entity: EcsEntity,
    pub vel: Vec3<f32>,
    pub surface_normal: Vec3<f32>,
}

pub struct SetLanternEvent(pub EcsEntity, pub bool);

pub struct NpcInteractEvent(pub EcsEntity, pub EcsEntity, pub Subject);

pub struct DialogueEvent(pub EcsEntity, pub EcsEntity, pub rtsim::Dialogue);

pub struct InviteResponseEvent(pub EcsEntity, pub InviteResponse);

pub struct InitiateInviteEvent(pub EcsEntity, pub Uid, pub InviteKind);

pub struct ProcessTradeActionEvent(pub EcsEntity, pub TradeId, pub TradeAction);

pub enum MountEvent {
    MountEntity(EcsEntity, EcsEntity),
    MountVolume(EcsEntity, VolumePos),
    Unmount(EcsEntity),
}

pub struct SetPetStayEvent(pub EcsEntity, pub EcsEntity, pub bool);

pub struct PossessEvent(pub Uid, pub Uid);

pub struct TransformEvent {
    pub target_entity: Uid,
    pub entity_info: EntityInfo,
    /// If set to false, players wont be transformed unless with a Possessor
    /// presence kind
    pub allow_players: bool,
    /// Whether the entity should be deleted if transforming fails (only applies
    /// to non-players)
    pub delete_on_failure: bool,
}

pub struct StartInteractionEvent(pub Interaction);

pub struct AuraEvent {
    pub entity: EcsEntity,
    pub aura_change: comp::AuraChange,
}

pub struct BuffEvent {
    pub entity: EcsEntity,
    pub buff_change: comp::BuffChange,
}

pub struct EnergyChangeEvent {
    pub entity: EcsEntity,
    pub change: f32,
    pub reset_rate: bool,
}

pub struct ComboChangeEvent {
    pub entity: EcsEntity,
    pub change: i32,
}

pub struct ParryHookEvent {
    pub defender: EcsEntity,
    pub attacker: Option<EcsEntity>,
    pub source: AttackSource,
    pub poise_multiplier: f32,
}

/// Attempt to mine a block, turning it into an item.
pub struct MineBlockEvent {
    pub entity: EcsEntity,
    pub pos: Vec3<i32>,
    pub tool: Option<comp::tool::ToolKind>,
}

pub struct TeleportToEvent {
    pub entity: EcsEntity,
    pub target: Uid,
    pub max_range: Option<f32>,
}

pub struct SoundEvent {
    pub sound: Sound,
}

pub struct CreateSpriteEvent {
    pub pos: Vec3<i32>,
    pub sprite: SpriteKind,
    pub del_timeout: Option<(f32, f32)>,
}

pub struct EntityAttackedHookEvent {
    pub entity: EcsEntity,
    pub attacker: Option<EcsEntity>,
}

pub struct ChangeAbilityEvent {
    pub entity: EcsEntity,
    pub slot: usize,
    pub auxiliary_key: comp::ability::AuxiliaryKey,
    pub new_ability: comp::ability::AuxiliaryAbility,
}

pub struct ChangeStanceEvent {
    pub entity: EcsEntity,
    pub stance: comp::Stance,
}

pub struct ChangeBodyEvent {
    pub entity: EcsEntity,
    pub new_body: comp::Body,
}

pub struct RemoveLightEmitterEvent {
    pub entity: EcsEntity,
}

pub struct StartTeleportingEvent {
    pub entity: EcsEntity,
    pub portal: EcsEntity,
}

pub struct ToggleSpriteLightEvent {
    pub entity: EcsEntity,
    pub pos: Vec3<i32>,
    pub enable: bool,
}

pub struct RegrowHeadEvent {
    pub entity: EcsEntity,
}

struct EventBusInner<E> {
    queue: VecDeque<E>,
    /// Saturates to u8::MAX and is never reset.
    ///
    /// Used in the first tick to check for if certain event types are handled
    /// and only handled once.
    #[cfg(debug_assertions)]
    recv_count: u8,
}

pub struct EventBus<E> {
    inner: Mutex<EventBusInner<E>>,
}

impl<E> Default for EventBus<E> {
    fn default() -> Self {
        Self {
            inner: Mutex::new(EventBusInner {
                queue: VecDeque::new(),
                #[cfg(debug_assertions)]
                recv_count: 0,
            }),
        }
    }
}

impl<E> EventBus<E> {
    pub fn emitter(&self) -> Emitter<E> {
        Emitter {
            bus: self,
            events: VecDeque::new(),
        }
    }

    pub fn emit_now(&self, event: E) {
        self.inner.lock().expect("Poisoned").queue.push_back(event);
    }

    pub fn recv_all(&self) -> impl ExactSizeIterator<Item = E> + use<E> {
        {
            let mut guard = self.inner.lock().expect("Poisoned");
            #[cfg(debug_assertions)]
            {
                guard.recv_count = guard.recv_count.saturating_add(1);
            }
            core::mem::take(&mut guard.queue)
        }
        .into_iter()
    }

    pub fn recv_all_mut(&mut self) -> impl ExactSizeIterator<Item = E> + use<E> {
        let inner = self.inner.get_mut().expect("Poisoned");
        #[cfg(debug_assertions)]
        {
            inner.recv_count = inner.recv_count.saturating_add(1);
        }
        core::mem::take(&mut inner.queue).into_iter()
    }

    #[cfg(debug_assertions)]
    pub fn recv_count(&mut self) -> u8 { self.inner.get_mut().expect("Poisoned").recv_count }
}

pub struct Emitter<'a, E> {
    bus: &'a EventBus<E>,
    pub events: VecDeque<E>,
}

impl<E> Emitter<'_, E> {
    pub fn emit(&mut self, event: E) { self.events.push_back(event); }

    pub fn emit_many(&mut self, events: impl IntoIterator<Item = E>) { self.events.extend(events); }

    pub fn append(&mut self, other: &mut VecDeque<E>) { self.events.append(other) }

    pub fn append_vec(&mut self, vec: Vec<E>) {
        if self.events.is_empty() {
            self.events = vec.into();
        } else {
            self.events.extend(vec);
        }
    }
}

impl<E> Drop for Emitter<'_, E> {
    fn drop(&mut self) {
        if !self.events.is_empty() {
            let mut guard = self.bus.inner.lock().expect("Poision");
            guard.queue.append(&mut self.events);
        }
    }
}

pub trait EmitExt<E> {
    fn emit(&mut self, event: E);
    fn emit_many(&mut self, events: impl IntoIterator<Item = E>);
}

/// Define ecs read data for event busses. And a way to convert them all to
/// emitters.
///
/// # Example:
/// ```
/// mod some_mod_is_necessary_for_the_test {
///     use veloren_common::event_emitters;
///     pub struct Foo;
///     pub struct Bar;
///     pub struct Baz;
///     event_emitters!(
///       pub struct ReadEvents[EventEmitters] {
///           foo: Foo, bar: Bar, baz: Baz,
///       }
///     );
/// }
/// ```
#[macro_export]
macro_rules! event_emitters {
    ($($vis:vis struct $read_data:ident[$emitters:ident] { $($ev_ident:ident: $ty:ty),+ $(,)? })+) => {
        mod event_emitters {
            use super::*;
            use specs::shred;
            $(
            #[derive(specs::SystemData)]
            pub struct $read_data<'a> {
                $($ev_ident: Option<specs::Read<'a, $crate::event::EventBus<$ty>>>),+
            }

            impl<'a> $read_data<'a> {
                pub fn get_emitters(&self) -> $emitters {
                    $emitters {
                        $($ev_ident: self.$ev_ident.as_ref().map(|e| e.emitter())),+
                    }
                }
            }

            pub struct $emitters<'a> {
                $($ev_ident: Option<$crate::event::Emitter<'a, $ty>>),+
            }

            impl<'a> $emitters<'a> {
                #[expect(unused)]
                pub fn append(&mut self, mut other: Self) {
                    $(
                        self.$ev_ident.as_mut().zip(other.$ev_ident).map(|(a, mut b)| a.append(&mut b.events));
                    )+
                }
            }

            $(
                impl<'a> $crate::event::EmitExt<$ty> for $emitters<'a> {
                    fn emit(&mut self, event: $ty) { self.$ev_ident.as_mut().map(|e| e.emit(event)); }
                    fn emit_many(&mut self, events: impl IntoIterator<Item = $ty>) { self.$ev_ident.as_mut().map(|e| e.emit_many(events)); }
                }
            )+
            )+
        }
        $(
            $vis use event_emitters::{$read_data, $emitters};
        )+
    }
}
