use crate::{
    character::CharacterId,
    combat::AttackSource,
    comp::{
        self,
        agent::Sound,
        dialogue::Subject,
        invite::{InviteKind, InviteResponse},
        misc::PortalData,
        DisconnectReason, Ori, Pos,
    },
    lottery::LootSpec,
    mounting::VolumePos,
    outcome::Outcome,
    rtsim::{RtSimEntity, RtSimVehicle},
    terrain::SpriteKind,
    trade::{TradeAction, TradeId},
    uid::Uid,
    util::Dir,
    Explosion,
};
use serde::{Deserialize, Serialize};
use specs::Entity as EcsEntity;
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

#[allow(clippy::large_enum_variant)] // TODO: Pending review in #587
#[derive(strum::EnumDiscriminants)]
#[strum_discriminants(repr(usize))]
#[strum_discriminants(derive(strum::EnumVariantNames))]
pub enum ServerEvent {
    Explosion {
        pos: Vec3<f32>,
        explosion: Explosion,
        owner: Option<Uid>,
    },
    Bonk {
        pos: Vec3<f32>,
        owner: Option<Uid>,
        target: Option<Uid>,
    },
    HealthChange {
        entity: EcsEntity,
        change: comp::HealthChange,
    },
    PoiseChange {
        entity: EcsEntity,
        change: comp::PoiseChange,
    },
    Delete(EcsEntity),
    Destroy {
        entity: EcsEntity,
        cause: comp::HealthChange,
    },
    InventoryManip(EcsEntity, comp::InventoryManip),
    GroupManip(EcsEntity, comp::GroupManip),
    Respawn(EcsEntity),
    Shoot {
        entity: EcsEntity,
        pos: Pos,
        dir: Dir,
        body: comp::Body,
        light: Option<comp::LightEmitter>,
        projectile: comp::Projectile,
        speed: f32,
        object: Option<comp::Object>,
    },
    Shockwave {
        properties: comp::shockwave::Properties,
        pos: Pos,
        ori: Ori,
    },
    Knockback {
        entity: EcsEntity,
        impulse: Vec3<f32>,
    },
    BeamSegment {
        properties: comp::beam::Properties,
        pos: Pos,
        ori: Ori,
    },
    LandOnGround {
        entity: EcsEntity,
        vel: Vec3<f32>,
        surface_normal: Vec3<f32>,
    },
    EnableLantern(EcsEntity),
    DisableLantern(EcsEntity),
    NpcInteract(EcsEntity, EcsEntity, Subject),
    InviteResponse(EcsEntity, InviteResponse),
    InitiateInvite(EcsEntity, Uid, InviteKind),
    ProcessTradeAction(EcsEntity, TradeId, TradeAction),
    Mount(EcsEntity, EcsEntity),
    MountVolume(EcsEntity, VolumePos),
    Unmount(EcsEntity),
    SetPetStay(EcsEntity, EcsEntity, bool),
    Possess(Uid, Uid),
    /// Inserts default components for a character when loading into the game
    InitCharacterData {
        entity: EcsEntity,
        character_id: CharacterId,
        requested_view_distances: crate::ViewDistances,
    },
    InitSpectator(EcsEntity, crate::ViewDistances),
    UpdateCharacterData {
        entity: EcsEntity,
        components: (
            comp::Body,
            comp::Stats,
            comp::SkillSet,
            comp::Inventory,
            Option<comp::Waypoint>,
            Vec<(comp::Pet, comp::Body, comp::Stats)>,
            comp::ActiveAbilities,
            Option<comp::MapMarker>,
        ),
        metadata: UpdateCharacterMetadata,
    },
    ExitIngame {
        entity: EcsEntity,
    },
    // TODO: to avoid breakage when adding new fields, perhaps have an `NpcBuilder` type?
    CreateNpc {
        pos: Pos,
        npc: NpcBuilder,
    },
    CreateShip {
        pos: Pos,
        ori: Ori,
        ship: comp::ship::Body,
        rtsim_entity: Option<RtSimVehicle>,
        driver: Option<NpcBuilder>,
    },
    CreateWaypoint(Vec3<f32>),
    CreateTeleporter(Vec3<f32>, PortalData),
    ClientDisconnect(EcsEntity, DisconnectReason),
    ClientDisconnectWithoutPersistence(EcsEntity),
    Command(EcsEntity, String, Vec<String>),
    /// Send a chat message to the player from an npc or other player
    Chat(comp::UnresolvedChatMsg),
    Aura {
        entity: EcsEntity,
        aura_change: comp::AuraChange,
    },
    Buff {
        entity: EcsEntity,
        buff_change: comp::BuffChange,
    },
    EnergyChange {
        entity: EcsEntity,
        change: f32,
    },
    ComboChange {
        entity: EcsEntity,
        change: i32,
    },
    ParryHook {
        defender: EcsEntity,
        attacker: Option<EcsEntity>,
        source: AttackSource,
    },
    RequestSiteInfo {
        entity: EcsEntity,
        id: SiteId,
    },
    // Attempt to mine a block, turning it into an item
    MineBlock {
        entity: EcsEntity,
        pos: Vec3<i32>,
        tool: Option<comp::tool::ToolKind>,
    },
    TeleportTo {
        entity: EcsEntity,
        target: Uid,
        max_range: Option<f32>,
    },
    CreateSafezone {
        range: Option<f32>,
        pos: Pos,
    },
    Sound {
        sound: Sound,
    },
    CreateSprite {
        pos: Vec3<i32>,
        sprite: SpriteKind,
        del_timeout: Option<(f32, f32)>,
    },
    TamePet {
        pet_entity: EcsEntity,
        owner_entity: EcsEntity,
    },
    EntityAttackedHook {
        entity: EcsEntity,
        attacker: Option<EcsEntity>,
    },
    ChangeAbility {
        entity: EcsEntity,
        slot: usize,
        auxiliary_key: comp::ability::AuxiliaryKey,
        new_ability: comp::ability::AuxiliaryAbility,
    },
    UpdateMapMarker {
        entity: EcsEntity,
        update: comp::MapMarkerChange,
    },
    MakeAdmin {
        entity: EcsEntity,
        admin: comp::Admin,
        uuid: Uuid,
    },
    DeleteCharacter {
        entity: EcsEntity,
        requesting_player_uuid: String,
        character_id: CharacterId,
    },
    ChangeStance {
        entity: EcsEntity,
        stance: comp::Stance,
    },
    ChangeBody {
        entity: EcsEntity,
        new_body: comp::Body,
    },
    RemoveLightEmitter {
        entity: EcsEntity,
    },
    TeleportToPosition {
        entity: EcsEntity,
        position: Vec3<f32>,
    },
    StartTeleporting {
        entity: EcsEntity,
        portal: EcsEntity,
    },
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
}

pub struct Emitter<'a, E> {
    bus: &'a EventBus<E>,
    events: VecDeque<E>,
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
