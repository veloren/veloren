use crate::{
    character::CharacterId,
    combat::AttackSource,
    comp::{
        self,
        agent::Sound,
        dialogue::Subject,
        invite::{InviteKind, InviteResponse},
        misc::PortalData,
        DisconnectReason, LootOwner, Ori, Pos, UnresolvedChatMsg, Vel,
    },
    lottery::LootSpec,
    mounting::VolumePos,
    outcome::Outcome,
    rtsim::RtSimEntity,
    terrain::SpriteKind,
    trade::{TradeAction, TradeId},
    uid::Uid,
    util::Dir,
    Explosion,
};
use serde::{Deserialize, Serialize};
use specs::{Entity as EcsEntity, World};
use std::{collections::VecDeque, ops::DerefMut, sync::Mutex};
use uuid::Uuid;
use vek::*;

pub type SiteId = u64;

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
    pub rtsim_entity: Option<RtSimEntity>,
    pub projectile: Option<comp::Projectile>,
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
        }
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
}

pub struct ClientConnectedEvent {
    pub entity: EcsEntity,
}
pub struct ClientDisconnectEvent(pub EcsEntity, pub DisconnectReason);
pub struct ClientDisconnectWithoutPersistenceEvent(pub EcsEntity);

pub struct ChatEvent(pub UnresolvedChatMsg);
pub struct CommandEvent(pub EcsEntity, pub String, pub Vec<String>);

// Entity Creation
pub struct CreateWaypointEvent(pub Vec3<f32>);
pub struct CreateTeleporterEvent(pub Vec3<f32>, pub PortalData);

pub struct CreateNpcEvent {
    pub pos: Pos,
    pub ori: Ori,
    pub npc: NpcBuilder,
    pub rider: Option<NpcBuilder>,
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
    pub item: comp::Item,
    pub loot_owner: Option<LootOwner>,
}
pub struct CreateObjectEvent {
    pub pos: Pos,
    pub vel: Vel,
    pub body: comp::object::Body,
    pub object: Option<comp::Object>,
    pub item: Option<comp::Item>,
    pub light_emitter: Option<comp::LightEmitter>,
    pub stats: Option<comp::Stats>,
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

pub struct InviteResponseEvent(pub EcsEntity, pub InviteResponse);

pub struct InitiateInviteEvent(pub EcsEntity, pub Uid, pub InviteKind);

pub struct ProcessTradeActionEvent(pub EcsEntity, pub TradeId, pub TradeAction);

pub struct MountEvent(pub EcsEntity, pub EcsEntity);

pub struct MountVolumeEvent(pub EcsEntity, pub VolumePos);

pub struct UnmountEvent(pub EcsEntity);

pub struct SetPetStayEvent(pub EcsEntity, pub EcsEntity, pub bool);

pub struct PossessEvent(pub Uid, pub Uid);

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
}

pub struct ComboChangeEvent {
    pub entity: EcsEntity,
    pub change: i32,
}

pub struct ParryHookEvent {
    pub defender: EcsEntity,
    pub attacker: Option<EcsEntity>,
    pub source: AttackSource,
}

pub struct RequestSiteInfoEvent {
    pub entity: EcsEntity,
    pub id: SiteId,
}

// Attempt to mine a block, turning it into an item
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

pub struct CreateSafezoneEvent {
    pub range: Option<f32>,
    pub pos: Pos,
}

pub struct SoundEvent {
    pub sound: Sound,
}

pub struct CreateSpriteEvent {
    pub pos: Vec3<i32>,
    pub sprite: SpriteKind,
    pub del_timeout: Option<(f32, f32)>,
}

pub struct TamePetEvent {
    pub pet_entity: EcsEntity,
    pub owner_entity: EcsEntity,
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

pub struct TeleportToPositionEvent {
    pub entity: EcsEntity,
    pub position: Vec3<f32>,
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

pub struct EventBus<E> {
    queue: Mutex<VecDeque<E>>,
}

impl<E> Default for EventBus<E> {
    fn default() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
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

    pub fn emit_now(&self, event: E) { self.queue.lock().unwrap().push_back(event); }

    pub fn recv_all(&self) -> impl ExactSizeIterator<Item = E> {
        std::mem::take(self.queue.lock().unwrap().deref_mut()).into_iter()
    }

    pub fn recv_all_mut(&mut self) -> impl ExactSizeIterator<Item = E> {
        std::mem::take(self.queue.get_mut().unwrap()).into_iter()
    }
}

pub struct Emitter<'a, E> {
    bus: &'a EventBus<E>,
    pub events: VecDeque<E>,
}

impl<'a, E> Emitter<'a, E> {
    pub fn emit(&mut self, event: E) { self.events.push_back(event); }

    pub fn emit_many(&mut self, events: impl IntoIterator<Item = E>) { self.events.extend(events); }

    pub fn append(&mut self, other: &mut VecDeque<E>) { self.events.append(other) }

    // TODO: allow just emitting the whole vec of events at once? without copying
    pub fn append_vec(&mut self, vec: Vec<E>) { self.events.extend(vec) }
}

impl<'a, E> Drop for Emitter<'a, E> {
    fn drop(&mut self) {
        if !self.events.is_empty() {
            self.bus.queue.lock().unwrap().append(&mut self.events);
        }
    }
}

pub trait EmitExt<E> {
    fn emit(&mut self, event: E);
    fn emit_many(&mut self, events: impl IntoIterator<Item = E>);
}

pub fn register_event_busses(ecs: &mut World) {
    ecs.insert(EventBus::<ClientConnectedEvent>::default());
    ecs.insert(EventBus::<ClientDisconnectEvent>::default());
    ecs.insert(EventBus::<ClientDisconnectWithoutPersistenceEvent>::default());
    ecs.insert(EventBus::<ChatEvent>::default());
    ecs.insert(EventBus::<CommandEvent>::default());
    ecs.insert(EventBus::<CreateWaypointEvent>::default());
    ecs.insert(EventBus::<CreateTeleporterEvent>::default());
    ecs.insert(EventBus::<CreateNpcEvent>::default());
    ecs.insert(EventBus::<CreateShipEvent>::default());
    ecs.insert(EventBus::<CreateItemDropEvent>::default());
    ecs.insert(EventBus::<CreateObjectEvent>::default());
    ecs.insert(EventBus::<ExplosionEvent>::default());
    ecs.insert(EventBus::<BonkEvent>::default());
    ecs.insert(EventBus::<HealthChangeEvent>::default());
    ecs.insert(EventBus::<PoiseChangeEvent>::default());
    ecs.insert(EventBus::<DeleteEvent>::default());
    ecs.insert(EventBus::<DestroyEvent>::default());
    ecs.insert(EventBus::<InventoryManipEvent>::default());
    ecs.insert(EventBus::<GroupManipEvent>::default());
    ecs.insert(EventBus::<RespawnEvent>::default());
    ecs.insert(EventBus::<ShootEvent>::default());
    ecs.insert(EventBus::<ShockwaveEvent>::default());
    ecs.insert(EventBus::<KnockbackEvent>::default());
    ecs.insert(EventBus::<LandOnGroundEvent>::default());
    ecs.insert(EventBus::<SetLanternEvent>::default());
    ecs.insert(EventBus::<NpcInteractEvent>::default());
    ecs.insert(EventBus::<InviteResponseEvent>::default());
    ecs.insert(EventBus::<InitiateInviteEvent>::default());
    ecs.insert(EventBus::<ProcessTradeActionEvent>::default());
    ecs.insert(EventBus::<MountEvent>::default());
    ecs.insert(EventBus::<MountVolumeEvent>::default());
    ecs.insert(EventBus::<UnmountEvent>::default());
    ecs.insert(EventBus::<SetPetStayEvent>::default());
    ecs.insert(EventBus::<PossessEvent>::default());
    ecs.insert(EventBus::<InitializeCharacterEvent>::default());
    ecs.insert(EventBus::<InitializeSpectatorEvent>::default());
    ecs.insert(EventBus::<UpdateCharacterDataEvent>::default());
    ecs.insert(EventBus::<ExitIngameEvent>::default());
    ecs.insert(EventBus::<AuraEvent>::default());
    ecs.insert(EventBus::<BuffEvent>::default());
    ecs.insert(EventBus::<EnergyChangeEvent>::default());
    ecs.insert(EventBus::<ComboChangeEvent>::default());
    ecs.insert(EventBus::<ParryHookEvent>::default());
    ecs.insert(EventBus::<RequestSiteInfoEvent>::default());
    ecs.insert(EventBus::<MineBlockEvent>::default());
    ecs.insert(EventBus::<TeleportToEvent>::default());
    ecs.insert(EventBus::<CreateSafezoneEvent>::default());
    ecs.insert(EventBus::<SoundEvent>::default());
    ecs.insert(EventBus::<CreateSpriteEvent>::default());
    ecs.insert(EventBus::<TamePetEvent>::default());
    ecs.insert(EventBus::<EntityAttackedHookEvent>::default());
    ecs.insert(EventBus::<ChangeAbilityEvent>::default());
    ecs.insert(EventBus::<UpdateMapMarkerEvent>::default());
    ecs.insert(EventBus::<MakeAdminEvent>::default());
    ecs.insert(EventBus::<DeleteCharacterEvent>::default());
    ecs.insert(EventBus::<ChangeStanceEvent>::default());
    ecs.insert(EventBus::<ChangeBodyEvent>::default());
    ecs.insert(EventBus::<RemoveLightEmitterEvent>::default());
    ecs.insert(EventBus::<TeleportToPositionEvent>::default());
    ecs.insert(EventBus::<StartTeleportingEvent>::default());
    ecs.insert(EventBus::<ToggleSpriteLightEvent>::default());
}

/// Define ecs read data for event busses. And a way to convert them all to
/// emitters.
///
/// # Example:
/// ```
/// struct Foo;
/// struct Bar;
/// struct Baz;
/// event_emitters!(
///     pub struct ReadEvents[EventEmitters] {
///         foo: Foo, bar: Bar, baz: Baz,
///     }
/// );
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
                $($ev_ident: specs::Read<'a, $crate::event::EventBus<$ty>>),+
            }

            impl<'a> $read_data<'a> {
                #[allow(unused)]
                pub fn get_emitters(&self) -> $emitters {
                    $emitters {
                        $($ev_ident: self.$ev_ident.emitter()),+
                    }
                }
            }

            pub struct $emitters<'a> {
                $($ev_ident: $crate::event::Emitter<'a, $ty>),+
            }

            impl<'a> $emitters<'a> {
                #[allow(unused)]
                pub fn append(&mut self, mut other: Self) {
                    $(
                        self.$ev_ident.append(&mut other.$ev_ident.events);
                    )+
                }
            }

            $(
                impl<'a> $crate::event::EmitExt<$ty> for $emitters<'a> {
                    fn emit(&mut self, event: $ty) { self.$ev_ident.emit(event) }
                    fn emit_many(&mut self, events: impl IntoIterator<Item = $ty>) { self.$ev_ident.emit_many(events) }
                }
            )+
            )+
        }
        $(
            $vis use event_emitters::{$read_data, $emitters};
        )+
    }
}
