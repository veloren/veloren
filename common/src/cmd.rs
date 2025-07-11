use crate::{
    assets::{self, AssetCombined, Concatenate},
    combat::GroupTarget,
    comp::{
        self, AdminRole as Role, Skill, aura::AuraKindVariant, buff::BuffKind,
        inventory::item::try_all_item_defs,
    },
    generation::try_all_entity_configs,
    npc, outcome,
    recipe::RecipeBookManifest,
    spot::Spot,
    terrain,
};
use common_i18n::Content;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    str::FromStr,
};
use strum::{AsRefStr, EnumIter, EnumString, IntoEnumIterator, VariantNames};
use tracing::warn;

/// Struct representing a command that a user can run from server chat.
pub struct ChatCommandData {
    /// A list of arguments useful for both tab completion and parsing
    pub args: Vec<ArgumentSpec>,
    /// The i18n content for the description of the command
    pub description: Content,
    /// Whether the command requires administrator permissions.
    pub needs_role: Option<Role>,
}

impl ChatCommandData {
    pub fn new(args: Vec<ArgumentSpec>, description: Content, needs_role: Option<Role>) -> Self {
        Self {
            args,
            description,
            needs_role,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum KitSpec {
    Item(String),
    ModularWeaponSet {
        tool: comp::tool::ToolKind,
        material: comp::item::Material,
        hands: Option<comp::item::tool::Hands>,
    },
    ModularWeaponRandom {
        tool: comp::tool::ToolKind,
        material: comp::item::Material,
        hands: Option<comp::item::tool::Hands>,
    },
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct KitManifest(pub HashMap<String, Vec<(KitSpec, u32)>>);
impl assets::Asset for KitManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}
impl Concatenate for KitManifest {
    fn concatenate(self, b: Self) -> Self { KitManifest(self.0.concatenate(b.0)) }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct SkillPresetManifest(pub HashMap<String, Vec<(Skill, u8)>>);
impl assets::Asset for SkillPresetManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}
impl Concatenate for SkillPresetManifest {
    fn concatenate(self, b: Self) -> Self { SkillPresetManifest(self.0.concatenate(b.0)) }
}

pub const KIT_MANIFEST_PATH: &str = "server.manifests.kits";
pub const PRESET_MANIFEST_PATH: &str = "server.manifests.presets";

/// Enum for all possible area types
#[derive(Debug, Clone, EnumIter, EnumString, AsRefStr)]
pub enum AreaKind {
    #[strum(serialize = "build")]
    Build,
    #[strum(serialize = "no_durability")]
    NoDurability,
}

lazy_static! {
    static ref ALIGNMENTS: Vec<String> = ["wild", "enemy", "npc", "pet"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    static ref SKILL_TREES: Vec<String> = ["general", "sword", "axe", "hammer", "bow", "staff", "sceptre", "mining"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    /// TODO: Make this use hot-reloading
    pub static ref ENTITIES: Vec<String> = {
        let npc_names = &*npc::NPC_NAMES.read();

        // HashSets for deduplication of male, female, etc
        let mut categories = HashSet::new();
        let mut species = HashSet::new();
        for body in comp::Body::iter() {
            // plugin doesn't seem to be spawnable, yet
            if matches!(body, comp::Body::Plugin(_)) {
                continue;
            }

            if let Some(meta) = npc_names.get_species_meta(&body) {
                categories.insert(npc_names[&body].keyword.clone());
                species.insert(meta.keyword.clone());
            }
        }

        let mut strings = Vec::new();
        strings.extend(categories);
        strings.extend(species);

        strings
    };
    static ref AREA_KINDS: Vec<String> = AreaKind::iter().map(|kind| kind.as_ref().to_string()).collect();
    static ref OBJECTS: Vec<String> = comp::object::ALL_OBJECTS
        .iter()
        .map(|o| o.to_string().to_string())
        .collect();
    static ref RECIPES: Vec<String> = {
        let rbm = RecipeBookManifest::load().cloned();
        rbm.keys().cloned().collect::<Vec<String>>()
    };
    static ref TIMES: Vec<String> = [
        "midnight", "night", "dawn", "morning", "day", "noon", "dusk"
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    static ref WEATHERS: Vec<String> = [
        "clear", "cloudy", "rain", "wind", "storm"
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    pub static ref BUFF_PARSER: HashMap<String, BuffKind> = {
        let string_from_buff = |kind| match kind {
            BuffKind::Burning => "burning",
            BuffKind::Regeneration => "regeneration",
            BuffKind::Saturation => "saturation",
            BuffKind::Bleeding => "bleeding",
            BuffKind::Cursed => "cursed",
            BuffKind::Potion => "potion",
            BuffKind::Agility => "agility",
            BuffKind::RestingHeal => "resting_heal",
            BuffKind::EnergyRegen => "energy_regen",
            BuffKind::ComboGeneration => "combo_generation",
            BuffKind::IncreaseMaxEnergy => "increase_max_energy",
            BuffKind::IncreaseMaxHealth => "increase_max_health",
            BuffKind::Invulnerability => "invulnerability",
            BuffKind::ProtectingWard => "protecting_ward",
            BuffKind::Frenzied => "frenzied",
            BuffKind::Crippled => "crippled",
            BuffKind::Frozen => "frozen",
            BuffKind::Wet => "wet",
            BuffKind::Ensnared => "ensnared",
            BuffKind::Poisoned => "poisoned",
            BuffKind::Hastened => "hastened",
            BuffKind::Fortitude => "fortitude",
            BuffKind::Parried => "parried",
            BuffKind::PotionSickness => "potion_sickness",
            BuffKind::Reckless => "reckless",
            BuffKind::Polymorphed => "polymorphed",
            BuffKind::Flame => "flame",
            BuffKind::Frigid => "frigid",
            BuffKind::Lifesteal => "lifesteal",
            // BuffKind::SalamanderAspect => "salamander_aspect",
            BuffKind::ImminentCritical => "imminent_critical",
            BuffKind::Fury => "fury",
            BuffKind::Sunderer => "sunderer",
            BuffKind::Defiance => "defiance",
            BuffKind::Bloodfeast => "bloodfeast",
            BuffKind::Berserk => "berserk",
            BuffKind::Heatstroke => "heatstroke",
            BuffKind::ScornfulTaunt => "scornful_taunt",
            BuffKind::Rooted => "rooted",
            BuffKind::Winded => "winded",
            BuffKind::Amnesia => "amnesia",
            BuffKind::OffBalance => "off_balance",
            BuffKind::Tenacity => "tenacity",
            BuffKind::Resilience => "resilience",
        };
        let mut buff_parser = HashMap::new();
        for kind in BuffKind::iter() {
            buff_parser.insert(string_from_buff(kind).to_string(), kind);
        }
        buff_parser
    };

    pub static ref BUFF_PACK: Vec<String> = {
        let mut buff_pack: Vec<_> = BUFF_PARSER.keys().cloned().collect();
        // Remove invulnerability as it removes debuffs
        buff_pack.retain(|kind| kind != "invulnerability");
        buff_pack
    };

    static ref BUFFS: Vec<String> = {
        let mut buff_pack: Vec<String> = BUFF_PARSER.keys().cloned().collect();

        // Add `all` and `clear` as valid command
        buff_pack.push("all".to_owned());
        buff_pack.push("clear".to_owned());
        buff_pack
    };

    static ref BLOCK_KINDS: Vec<String> = terrain::block::BlockKind::iter()
        .map(|bk| bk.to_string())
        .collect();

    static ref SPRITE_KINDS: Vec<String> = terrain::sprite::SPRITE_KINDS
        .keys()
        .cloned()
        .collect();

    static ref OUTCOME_KINDS: Vec<String> = outcome::Outcome::VARIANTS
        .iter()
        .map(|s| s.to_string())
        .collect();

    static ref ROLES: Vec<String> = ["admin", "moderator"].iter().copied().map(Into::into).collect();

    /// List of item's asset specifiers. Useful for tab completing.
    /// Doesn't cover all items (like modulars), includes "fake" items like
    /// TagExamples.
    pub static ref ITEM_SPECS: Vec<String> = {
        let mut items = try_all_item_defs()
            .unwrap_or_else(|e| {
                warn!(?e, "Failed to load item specifiers");
                Vec::new()
            });
        items.sort();
        items
    };

    /// List of all entity configs. Useful for tab completing
    pub static ref ENTITY_CONFIGS: Vec<String> = {
        try_all_entity_configs()
            .unwrap_or_else(|e| {
                warn!(?e, "Failed to load entity configs");
                Vec::new()
            })
    };

    pub static ref KITS: Vec<String> = {
        let mut kits = if let Ok(kits) = KitManifest::load_and_combine_static(KIT_MANIFEST_PATH) {
            let mut kits = kits.read().0.keys().cloned().collect::<Vec<String>>();
            kits.sort();
            kits
        } else {
            Vec::new()
        };
        kits.push("all".to_owned());

        kits
    };

    static ref PRESETS: HashMap<String, Vec<(Skill, u8)>> = {
        if let Ok(presets) = SkillPresetManifest::load_and_combine_static(PRESET_MANIFEST_PATH) {
            presets.read().0.clone()
        } else {
            warn!("Error while loading presets");
            HashMap::new()
        }
    };

    static ref PRESET_LIST: Vec<String> = {
        let mut preset_list: Vec<String> = PRESETS.keys().cloned().collect();
        preset_list.push("clear".to_owned());

        preset_list
    };

    /// Map from string to a Spot's kind (except RonFile)
    pub static ref SPOT_PARSER: HashMap<String, Spot> = {
        let spot_to_string = |kind| match kind {
            Spot::DwarvenGrave => "dwarven_grave",
            Spot::SaurokAltar => "saurok_altar",
            Spot::MyrmidonTemple => "myrmidon_temple",
            Spot::GnarlingTotem => "gnarling_totem",
            Spot::WitchHouse => "witch_house",
            Spot::GnomeSpring => "gnome_spring",
            Spot::WolfBurrow => "wolf_burrow",
            Spot::Igloo => "igloo",
            Spot::LionRock => "lion_rock",
            Spot::TreeStumpForest => "tree_stump_forest",
            Spot::DesertBones => "desert_bones",
            Spot::Arch => "arch",
            Spot::AirshipCrash => "airship_crash",
            Spot::FruitTree => "fruit_tree",
            Spot::Shipwreck => "shipwreck",
            Spot::Shipwreck2 => "shipwreck2",
            Spot::FallenTree => "fallen_tree",
            Spot::GraveSmall => "grave_small",
            Spot::JungleTemple => "jungle_temple",
            Spot::SaurokTotem => "saurok_totem",
            Spot::JungleOutpost => "jungle_outpost",
            // unused here, but left for completeness
            Spot::RonFile(props) => &props.base_structures,
        };

        let mut map = HashMap::new();
        for spot_kind in Spot::iter() {
            map.insert(spot_to_string(spot_kind).to_owned(), spot_kind);
        }

        map
    };

    pub static ref SPOTS: Vec<String> = {
        let mut config_spots = crate::spot::RON_SPOT_PROPERTIES
            .0
            .iter()
            .map(|s| s.base_structures.clone())
            .collect::<Vec<_>>();

        config_spots.extend(SPOT_PARSER.keys().cloned());
        config_spots
    };
}

pub enum EntityTarget {
    Player(String),
    RtsimNpc(u64),
    Uid(crate::uid::Uid),
}

impl FromStr for EntityTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // NOTE: `@` is an invalid character in usernames, so we can use it here.
        if let Some((spec, data)) = s.split_once('@') {
            match spec {
                "rtsim" => Ok(EntityTarget::RtsimNpc(u64::from_str(data).map_err(
                    |_| format!("Expected a valid number after 'rtsim@' but found {data}."),
                )?)),
                "uid" => Ok(EntityTarget::Uid(
                    u64::from_str(data)
                        .map_err(|_| {
                            format!("Expected a valid number after 'uid@' but found {data}.")
                        })?
                        .into(),
                )),
                _ => Err(format!(
                    "Expected either 'rtsim' or 'uid' before '@' but found '{spec}'"
                )),
            }
        } else {
            Ok(EntityTarget::Player(s.to_string()))
        }
    }
}

// Please keep this sorted alphabetically :-)
#[derive(Copy, Clone, strum::EnumIter)]
pub enum ServerChatCommand {
    Adminify,
    Airship,
    Alias,
    AreaAdd,
    AreaList,
    AreaRemove,
    Aura,
    Ban,
    BanIp,
    BattleMode,
    BattleModeForce,
    Body,
    Buff,
    Build,
    Campfire,
    ClearPersistedTerrain,
    CreateLocation,
    DeathEffect,
    DebugColumn,
    DebugWays,
    DeleteLocation,
    DestroyTethers,
    DisconnectAllPlayers,
    Dismount,
    DropAll,
    Dummy,
    Explosion,
    Faction,
    GiveItem,
    Gizmos,
    GizmosRange,
    Goto,
    GotoRand,
    Group,
    GroupInvite,
    GroupKick,
    GroupLeave,
    GroupPromote,
    Health,
    IntoNpc,
    JoinFaction,
    Jump,
    Kick,
    Kill,
    KillNpcs,
    Kit,
    Lantern,
    Light,
    Lightning,
    Location,
    MakeBlock,
    MakeNpc,
    MakeSprite,
    MakeVolume,
    Motd,
    Mount,
    Object,
    Outcome,
    PermitBuild,
    Players,
    Poise,
    Portal,
    Region,
    ReloadChunks,
    RemoveLights,
    RepairEquipment,
    ResetRecipes,
    Respawn,
    RevokeBuild,
    RevokeBuildAll,
    RtsimChunk,
    RtsimInfo,
    RtsimNpc,
    RtsimPurge,
    RtsimTp,
    Safezone,
    Say,
    Scale,
    ServerPhysics,
    SetBodyType,
    SetMotd,
    SetWaypoint,
    Ship,
    Site,
    SkillPoint,
    SkillPreset,
    Spawn,
    Spot,
    Sudo,
    Tell,
    Tether,
    Time,
    TimeScale,
    Tp,
    Unban,
    UnbanIp,
    Version,
    WeatherZone,
    Whitelist,
    Wiring,
    World,
}

impl ServerChatCommand {
    pub fn data(&self) -> ChatCommandData {
        use ArgumentSpec::*;
        use Requirement::*;
        use Role::*;
        let cmd = ChatCommandData::new;
        match self {
            ServerChatCommand::Adminify => cmd(
                vec![PlayerName(Required), Enum("role", ROLES.clone(), Optional)],
                Content::localized("command-adminify-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Airship => cmd(
                vec![
                    Enum(
                        "kind",
                        comp::ship::ALL_AIRSHIPS
                            .iter()
                            .map(|b| format!("{b:?}"))
                            .collect(),
                        Optional,
                    ),
                    Float("destination_degrees_ccw_of_east", 90.0, Optional),
                ],
                Content::localized("command-airship-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Alias => cmd(
                vec![Any("name", Required)],
                Content::localized("command-alias-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::Aura => cmd(
                vec![
                    Float("aura_radius", 10.0, Required),
                    Float("aura_duration", 10.0, Optional),
                    Boolean("new_entity", "true".to_string(), Optional),
                    Enum("aura_target", GroupTarget::all_options(), Optional),
                    Enum("aura_kind", AuraKindVariant::all_options(), Required),
                    Any("aura spec", Optional),
                ],
                Content::localized("command-aura-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Buff => cmd(
                vec![
                    Enum("buff", BUFFS.clone(), Required),
                    Float("strength", 0.01, Optional),
                    Float("duration", 10.0, Optional),
                    Any("buff data spec", Optional),
                ],
                Content::localized("command-buff-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Ban => cmd(
                vec![
                    PlayerName(Required),
                    Boolean("overwrite", "true".to_string(), Optional),
                    Any("ban duration", Optional),
                    Message(Optional),
                ],
                Content::localized("command-ban-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::BanIp => cmd(
                vec![
                    PlayerName(Required),
                    Boolean("overwrite", "true".to_string(), Optional),
                    Any("ban duration", Optional),
                    Message(Optional),
                ],
                Content::localized("command-ban-ip-desc"),
                Some(Moderator),
            ),
            #[rustfmt::skip]
            ServerChatCommand::BattleMode => cmd(
                vec![Enum(
                    "battle mode",
                    vec!["pvp".to_owned(), "pve".to_owned()],
                    Optional,
                )],
                Content::localized("command-battlemode-desc"),
                None,

            ),
            ServerChatCommand::IntoNpc => cmd(
                vec![AssetPath(
                    "entity_config",
                    "common.entity.",
                    ENTITY_CONFIGS.clone(),
                    Required,
                )],
                Content::localized("command-into_npc-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Body => cmd(
                vec![Enum("body", ENTITIES.clone(), Required)],
                Content::localized("command-body-desc"),
                Some(Admin),
            ),
            ServerChatCommand::BattleModeForce => cmd(
                vec![Enum(
                    "battle mode",
                    vec!["pvp".to_owned(), "pve".to_owned()],
                    Required,
                )],
                Content::localized("command-battlemode_force-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Build => cmd(vec![], Content::localized("command-build-desc"), None),
            ServerChatCommand::AreaAdd => cmd(
                vec![
                    Any("name", Required),
                    Enum("kind", AREA_KINDS.clone(), Required),
                    Integer("xlo", 0, Required),
                    Integer("xhi", 10, Required),
                    Integer("ylo", 0, Required),
                    Integer("yhi", 10, Required),
                    Integer("zlo", 0, Required),
                    Integer("zhi", 10, Required),
                ],
                Content::localized("command-area_add-desc"),
                Some(Admin),
            ),
            ServerChatCommand::AreaList => cmd(
                vec![],
                Content::localized("command-area_list-desc"),
                Some(Admin),
            ),
            ServerChatCommand::AreaRemove => cmd(
                vec![
                    Any("name", Required),
                    Enum("kind", AREA_KINDS.clone(), Required),
                ],
                Content::localized("command-area_remove-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Campfire => cmd(
                vec![],
                Content::localized("command-campfire-desc"),
                Some(Admin),
            ),
            ServerChatCommand::ClearPersistedTerrain => cmd(
                vec![Integer("chunk_radius", 6, Required)],
                Content::localized("command-clear_persisted_terrain-desc"),
                Some(Admin),
            ),
            ServerChatCommand::DeathEffect => cmd(
                vec![
                    Enum("death_effect", vec!["transform".to_string()], Required),
                    // NOTE: I added this for QoL as transform is currently the only death effect
                    // and takes an asset path, when more on-death effects are added to the command
                    // remove this.
                    AssetPath(
                        "entity_config",
                        "common.entity.",
                        ENTITY_CONFIGS.clone(),
                        Required,
                    ),
                ],
                Content::localized("command-death_effect-dest"),
                Some(Admin),
            ),
            ServerChatCommand::DebugColumn => cmd(
                vec![Integer("x", 15000, Required), Integer("y", 15000, Required)],
                Content::localized("command-debug_column-desc"),
                Some(Admin),
            ),
            ServerChatCommand::DebugWays => cmd(
                vec![Integer("x", 15000, Required), Integer("y", 15000, Required)],
                Content::localized("command-debug_ways-desc"),
                Some(Admin),
            ),
            ServerChatCommand::DisconnectAllPlayers => cmd(
                vec![Any("confirm", Required)],
                Content::localized("command-disconnect_all_players-desc"),
                Some(Admin),
            ),
            ServerChatCommand::DropAll => cmd(
                vec![],
                Content::localized("command-dropall-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::Dummy => cmd(
                vec![],
                Content::localized("command-dummy-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Explosion => cmd(
                vec![Float("radius", 5.0, Required)],
                Content::localized("command-explosion-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Faction => cmd(
                vec![Message(Optional)],
                Content::localized("command-faction-desc"),
                None,
            ),
            ServerChatCommand::GiveItem => cmd(
                vec![
                    AssetPath("item", "common.items.", ITEM_SPECS.clone(), Required),
                    Integer("num", 1, Optional),
                ],
                Content::localized("command-give_item-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Gizmos => cmd(
                vec![
                    Enum(
                        "kind",
                        ["All".to_string(), "None".to_string()]
                            .into_iter()
                            .chain(
                                comp::gizmos::GizmoSubscription::iter()
                                    .map(|kind| kind.to_string()),
                            )
                            .collect(),
                        Required,
                    ),
                    EntityTarget(Optional),
                ],
                Content::localized("command-gizmos-desc"),
                Some(Admin),
            ),
            ServerChatCommand::GizmosRange => cmd(
                vec![Float("range", 32.0, Required)],
                Content::localized("command-gizmos_range-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Goto => cmd(
                vec![
                    Float("x", 0.0, Required),
                    Float("y", 0.0, Required),
                    Float("z", 0.0, Required),
                    Boolean("Dismount from ship", "true".to_string(), Optional),
                ],
                Content::localized("command-goto-desc"),
                Some(Admin),
            ),
            ServerChatCommand::GotoRand => cmd(
                vec![Boolean("Dismount from ship", "true".to_string(), Optional)],
                Content::localized("command-goto-rand"),
                Some(Admin),
            ),
            ServerChatCommand::Group => cmd(
                vec![Message(Optional)],
                Content::localized("command-group-desc"),
                None,
            ),
            ServerChatCommand::GroupInvite => cmd(
                vec![PlayerName(Required)],
                Content::localized("command-group_invite-desc"),
                None,
            ),
            ServerChatCommand::GroupKick => cmd(
                vec![PlayerName(Required)],
                Content::localized("command-group_kick-desc"),
                None,
            ),
            ServerChatCommand::GroupLeave => {
                cmd(vec![], Content::localized("command-group_leave-desc"), None)
            },
            ServerChatCommand::GroupPromote => cmd(
                vec![PlayerName(Required)],
                Content::localized("command-group_promote-desc"),
                None,
            ),
            ServerChatCommand::Health => cmd(
                vec![Integer("hp", 100, Required)],
                Content::localized("command-health-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Respawn => cmd(
                vec![],
                Content::localized("command-respawn-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::JoinFaction => cmd(
                vec![Any("faction", Optional)],
                Content::localized("command-join_faction-desc"),
                None,
            ),
            ServerChatCommand::Jump => cmd(
                vec![
                    Float("x", 0.0, Required),
                    Float("y", 0.0, Required),
                    Float("z", 0.0, Required),
                    Boolean("Dismount from ship", "true".to_string(), Optional),
                ],
                Content::localized("command-jump-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Kick => cmd(
                vec![PlayerName(Required), Message(Optional)],
                Content::localized("command-kick-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::Kill => cmd(vec![], Content::localized("command-kill-desc"), None),
            ServerChatCommand::KillNpcs => cmd(
                vec![Float("radius", 100.0, Optional), Flag("--also-pets")],
                Content::localized("command-kill_npcs-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Kit => cmd(
                vec![Enum("kit_name", KITS.to_vec(), Required)],
                Content::localized("command-kit-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Lantern => cmd(
                vec![
                    Float("strength", 5.0, Required),
                    Float("r", 1.0, Optional),
                    Float("g", 1.0, Optional),
                    Float("b", 1.0, Optional),
                ],
                Content::localized("command-lantern-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Light => cmd(
                vec![
                    Float("r", 1.0, Optional),
                    Float("g", 1.0, Optional),
                    Float("b", 1.0, Optional),
                    Float("x", 0.0, Optional),
                    Float("y", 0.0, Optional),
                    Float("z", 0.0, Optional),
                    Float("strength", 5.0, Optional),
                ],
                Content::localized("command-light-desc"),
                Some(Admin),
            ),
            ServerChatCommand::MakeBlock => cmd(
                vec![
                    Enum("block", BLOCK_KINDS.clone(), Required),
                    Integer("r", 255, Optional),
                    Integer("g", 255, Optional),
                    Integer("b", 255, Optional),
                ],
                Content::localized("command-make_block-desc"),
                Some(Admin),
            ),
            ServerChatCommand::MakeNpc => cmd(
                vec![
                    AssetPath(
                        "entity_config",
                        "common.entity.",
                        ENTITY_CONFIGS.clone(),
                        Required,
                    ),
                    Integer("num", 1, Optional),
                ],
                Content::localized("command-make_npc-desc"),
                Some(Admin),
            ),
            ServerChatCommand::MakeSprite => cmd(
                vec![Enum("sprite", SPRITE_KINDS.clone(), Required)],
                Content::localized("command-make_sprite-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Motd => cmd(vec![], Content::localized("command-motd-desc"), None),
            ServerChatCommand::Object => cmd(
                vec![Enum("object", OBJECTS.clone(), Required)],
                Content::localized("command-object-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Outcome => cmd(
                vec![Enum("outcome", OUTCOME_KINDS.clone(), Required)],
                Content::localized("command-outcome-desc"),
                Some(Admin),
            ),
            ServerChatCommand::PermitBuild => cmd(
                vec![Any("area_name", Required)],
                Content::localized("command-permit_build-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Players => {
                cmd(vec![], Content::localized("command-players-desc"), None)
            },
            ServerChatCommand::Poise => cmd(
                vec![Integer("poise", 100, Required)],
                Content::localized("command-poise-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Portal => cmd(
                vec![
                    Float("x", 0., Required),
                    Float("y", 0., Required),
                    Float("z", 0., Required),
                    Boolean("requires_no_aggro", "true".to_string(), Optional),
                    Float("buildup_time", 5., Optional),
                ],
                Content::localized("command-portal-desc"),
                Some(Admin),
            ),
            ServerChatCommand::ReloadChunks => cmd(
                vec![Integer("chunk_radius", 6, Optional)],
                Content::localized("command-reload_chunks-desc"),
                Some(Admin),
            ),
            ServerChatCommand::ResetRecipes => cmd(
                vec![],
                Content::localized("command-reset_recipes-desc"),
                Some(Admin),
            ),
            ServerChatCommand::RemoveLights => cmd(
                vec![Float("radius", 20.0, Optional)],
                Content::localized("command-remove_lights-desc"),
                Some(Admin),
            ),
            ServerChatCommand::RevokeBuild => cmd(
                vec![Any("area_name", Required)],
                Content::localized("command-revoke_build-desc"),
                Some(Admin),
            ),
            ServerChatCommand::RevokeBuildAll => cmd(
                vec![],
                Content::localized("command-revoke_build_all-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Region => cmd(
                vec![Message(Optional)],
                Content::localized("command-region-desc"),
                None,
            ),
            ServerChatCommand::Safezone => cmd(
                vec![Float("range", 100.0, Optional)],
                Content::localized("command-safezone-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::Say => cmd(
                vec![Message(Optional)],
                Content::localized("command-say-desc"),
                None,
            ),
            ServerChatCommand::ServerPhysics => cmd(
                vec![
                    PlayerName(Required),
                    Boolean("enabled", "true".to_string(), Optional),
                    Message(Optional),
                ],
                Content::localized("command-server_physics-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::SetMotd => cmd(
                vec![Any("locale", Optional), Message(Optional)],
                Content::localized("command-set_motd-desc"),
                Some(Admin),
            ),
            ServerChatCommand::SetBodyType => cmd(
                vec![
                    Enum(
                        "body type",
                        vec!["Female".to_string(), "Male".to_string()],
                        Required,
                    ),
                    Boolean("permanent", "false".to_string(), Requirement::Optional),
                ],
                Content::localized("command-set_body_type-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Ship => cmd(
                vec![
                    Enum(
                        "kind",
                        comp::ship::ALL_SHIPS
                            .iter()
                            .map(|b| format!("{b:?}"))
                            .collect(),
                        Optional,
                    ),
                    Boolean(
                        "Whether the ship should be tethered to the target (or its mount)",
                        "false".to_string(),
                        Optional,
                    ),
                    Float("destination_degrees_ccw_of_east", 90.0, Optional),
                ],
                Content::localized("command-ship-desc"),
                Some(Admin),
            ),
            // Uses Message because site names can contain spaces,
            // which would be assumed to be separators otherwise
            ServerChatCommand::Site => cmd(
                vec![
                    SiteName(Required),
                    Boolean("Dismount from ship", "true".to_string(), Optional),
                ],
                Content::localized("command-site-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::SkillPoint => cmd(
                vec![
                    Enum("skill tree", SKILL_TREES.clone(), Required),
                    Integer("amount", 1, Optional),
                ],
                Content::localized("command-skill_point-desc"),
                Some(Admin),
            ),
            ServerChatCommand::SkillPreset => cmd(
                vec![Enum("preset_name", PRESET_LIST.to_vec(), Required)],
                Content::localized("command-skill_preset-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Spawn => cmd(
                vec![
                    Enum("alignment", ALIGNMENTS.clone(), Required),
                    Enum("entity", ENTITIES.clone(), Required),
                    Integer("amount", 1, Optional),
                    Boolean("ai", "true".to_string(), Optional),
                    Float("scale", 1.0, Optional),
                    Boolean("tethered", "false".to_string(), Optional),
                ],
                Content::localized("command-spawn-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Spot => cmd(
                vec![Enum("Spot kind to find", SPOTS.clone(), Required)],
                Content::localized("command-spot-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Sudo => cmd(
                vec![EntityTarget(Required), SubCommand],
                Content::localized("command-sudo-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::Tell => cmd(
                vec![PlayerName(Required), Message(Optional)],
                Content::localized("command-tell-desc"),
                None,
            ),
            ServerChatCommand::Time => cmd(
                vec![Enum("time", TIMES.clone(), Optional)],
                Content::localized("command-time-desc"),
                Some(Admin),
            ),
            ServerChatCommand::TimeScale => cmd(
                vec![Float("time scale", 1.0, Optional)],
                Content::localized("command-time_scale-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Tp => cmd(
                vec![
                    EntityTarget(Optional),
                    Boolean("Dismount from ship", "true".to_string(), Optional),
                ],
                Content::localized("command-tp-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::RtsimTp => cmd(
                vec![
                    Integer("npc index", 0, Required),
                    Boolean("Dismount from ship", "true".to_string(), Optional),
                ],
                Content::localized("command-rtsim_tp-desc"),
                Some(Admin),
            ),
            ServerChatCommand::RtsimInfo => cmd(
                vec![Integer("npc index", 0, Required)],
                Content::localized("command-rtsim_info-desc"),
                Some(Admin),
            ),
            ServerChatCommand::RtsimNpc => cmd(
                vec![Any("query", Required), Integer("max number", 20, Optional)],
                Content::localized("command-rtsim_npc-desc"),
                Some(Admin),
            ),
            ServerChatCommand::RtsimPurge => cmd(
                vec![Boolean(
                    "whether purging of rtsim data should occur on next startup",
                    true.to_string(),
                    Required,
                )],
                Content::localized("command-rtsim_purge-desc"),
                Some(Admin),
            ),
            ServerChatCommand::RtsimChunk => cmd(
                vec![],
                Content::localized("command-rtsim_chunk-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Unban => cmd(
                vec![PlayerName(Required)],
                Content::localized("command-unban-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::UnbanIp => cmd(
                vec![PlayerName(Required)],
                Content::localized("command-unban-ip-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::Version => {
                cmd(vec![], Content::localized("command-version-desc"), None)
            },
            ServerChatCommand::SetWaypoint => cmd(
                vec![],
                Content::localized("command-waypoint-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Wiring => cmd(
                vec![],
                Content::localized("command-wiring-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Whitelist => cmd(
                vec![Any("add/remove", Required), PlayerName(Required)],
                Content::localized("command-whitelist-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::World => cmd(
                vec![Message(Optional)],
                Content::localized("command-world-desc"),
                None,
            ),
            ServerChatCommand::MakeVolume => cmd(
                vec![Integer("size", 15, Optional)],
                Content::localized("command-make_volume-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Location => cmd(
                vec![Any("name", Required)],
                Content::localized("command-location-desc"),
                None,
            ),
            ServerChatCommand::CreateLocation => cmd(
                vec![Any("name", Required)],
                Content::localized("command-create_location-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::DeleteLocation => cmd(
                vec![Any("name", Required)],
                Content::localized("command-delete_location-desc"),
                Some(Moderator),
            ),
            ServerChatCommand::WeatherZone => cmd(
                vec![
                    Enum("weather kind", WEATHERS.clone(), Required),
                    Float("radius", 500.0, Optional),
                    Float("time", 300.0, Optional),
                ],
                Content::localized("command-weather_zone-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Lightning => cmd(
                vec![],
                Content::localized("command-lightning-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Scale => cmd(
                vec![
                    Float("factor", 1.0, Required),
                    Boolean("reset_mass", true.to_string(), Optional),
                ],
                Content::localized("command-scale-desc"),
                Some(Admin),
            ),
            ServerChatCommand::RepairEquipment => cmd(
                vec![ArgumentSpec::Boolean(
                    "repair inventory",
                    true.to_string(),
                    Optional,
                )],
                Content::localized("command-repair_equipment-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Tether => cmd(
                vec![
                    EntityTarget(Required),
                    Boolean("automatic length", "true".to_string(), Optional),
                ],
                Content::localized("command-tether-desc"),
                Some(Admin),
            ),
            ServerChatCommand::DestroyTethers => cmd(
                vec![],
                Content::localized("command-destroy_tethers-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Mount => cmd(
                vec![EntityTarget(Required)],
                Content::localized("command-mount-desc"),
                Some(Admin),
            ),
            ServerChatCommand::Dismount => cmd(
                vec![EntityTarget(Required)],
                Content::localized("command-dismount-desc"),
                Some(Admin),
            ),
        }
    }

    /// The keyword used to invoke the command, omitting the prefix.
    pub fn keyword(&self) -> &'static str {
        match self {
            ServerChatCommand::Adminify => "adminify",
            ServerChatCommand::Airship => "airship",
            ServerChatCommand::Alias => "alias",
            ServerChatCommand::AreaAdd => "area_add",
            ServerChatCommand::AreaList => "area_list",
            ServerChatCommand::AreaRemove => "area_remove",
            ServerChatCommand::Aura => "aura",
            ServerChatCommand::Ban => "ban",
            ServerChatCommand::BanIp => "ban_ip",
            ServerChatCommand::BattleMode => "battlemode",
            ServerChatCommand::BattleModeForce => "battlemode_force",
            ServerChatCommand::Body => "body",
            ServerChatCommand::Buff => "buff",
            ServerChatCommand::Build => "build",
            ServerChatCommand::Campfire => "campfire",
            ServerChatCommand::ClearPersistedTerrain => "clear_persisted_terrain",
            ServerChatCommand::DeathEffect => "death_effect",
            ServerChatCommand::DebugColumn => "debug_column",
            ServerChatCommand::DebugWays => "debug_ways",
            ServerChatCommand::DisconnectAllPlayers => "disconnect_all_players",
            ServerChatCommand::DropAll => "dropall",
            ServerChatCommand::Dummy => "dummy",
            ServerChatCommand::Explosion => "explosion",
            ServerChatCommand::Faction => "faction",
            ServerChatCommand::GiveItem => "give_item",
            ServerChatCommand::Gizmos => "gizmos",
            ServerChatCommand::GizmosRange => "gizmos_range",
            ServerChatCommand::Goto => "goto",
            ServerChatCommand::GotoRand => "goto_rand",
            ServerChatCommand::Group => "group",
            ServerChatCommand::GroupInvite => "group_invite",
            ServerChatCommand::GroupKick => "group_kick",
            ServerChatCommand::GroupLeave => "group_leave",
            ServerChatCommand::GroupPromote => "group_promote",
            ServerChatCommand::Health => "health",
            ServerChatCommand::IntoNpc => "into_npc",
            ServerChatCommand::JoinFaction => "join_faction",
            ServerChatCommand::Jump => "jump",
            ServerChatCommand::Kick => "kick",
            ServerChatCommand::Kill => "kill",
            ServerChatCommand::KillNpcs => "kill_npcs",
            ServerChatCommand::Kit => "kit",
            ServerChatCommand::Lantern => "lantern",
            ServerChatCommand::Respawn => "respawn",
            ServerChatCommand::Light => "light",
            ServerChatCommand::MakeBlock => "make_block",
            ServerChatCommand::MakeNpc => "make_npc",
            ServerChatCommand::MakeSprite => "make_sprite",
            ServerChatCommand::Motd => "motd",
            ServerChatCommand::Object => "object",
            ServerChatCommand::Outcome => "outcome",
            ServerChatCommand::PermitBuild => "permit_build",
            ServerChatCommand::Players => "players",
            ServerChatCommand::Poise => "poise",
            ServerChatCommand::Portal => "portal",
            ServerChatCommand::ResetRecipes => "reset_recipes",
            ServerChatCommand::Region => "region",
            ServerChatCommand::ReloadChunks => "reload_chunks",
            ServerChatCommand::RemoveLights => "remove_lights",
            ServerChatCommand::RevokeBuild => "revoke_build",
            ServerChatCommand::RevokeBuildAll => "revoke_build_all",
            ServerChatCommand::Safezone => "safezone",
            ServerChatCommand::Say => "say",
            ServerChatCommand::ServerPhysics => "server_physics",
            ServerChatCommand::SetMotd => "set_motd",
            ServerChatCommand::SetBodyType => "set_body_type",
            ServerChatCommand::Ship => "ship",
            ServerChatCommand::Site => "site",
            ServerChatCommand::SkillPoint => "skill_point",
            ServerChatCommand::SkillPreset => "skill_preset",
            ServerChatCommand::Spawn => "spawn",
            ServerChatCommand::Spot => "spot",
            ServerChatCommand::Sudo => "sudo",
            ServerChatCommand::Tell => "tell",
            ServerChatCommand::Time => "time",
            ServerChatCommand::TimeScale => "time_scale",
            ServerChatCommand::Tp => "tp",
            ServerChatCommand::RtsimTp => "rtsim_tp",
            ServerChatCommand::RtsimInfo => "rtsim_info",
            ServerChatCommand::RtsimNpc => "rtsim_npc",
            ServerChatCommand::RtsimPurge => "rtsim_purge",
            ServerChatCommand::RtsimChunk => "rtsim_chunk",
            ServerChatCommand::Unban => "unban",
            ServerChatCommand::UnbanIp => "unban_ip",
            ServerChatCommand::Version => "version",
            ServerChatCommand::SetWaypoint => "set_waypoint",
            ServerChatCommand::Wiring => "wiring",
            ServerChatCommand::Whitelist => "whitelist",
            ServerChatCommand::World => "world",
            ServerChatCommand::MakeVolume => "make_volume",
            ServerChatCommand::Location => "location",
            ServerChatCommand::CreateLocation => "create_location",
            ServerChatCommand::DeleteLocation => "delete_location",
            ServerChatCommand::WeatherZone => "weather_zone",
            ServerChatCommand::Lightning => "lightning",
            ServerChatCommand::Scale => "scale",
            ServerChatCommand::RepairEquipment => "repair_equipment",
            ServerChatCommand::Tether => "tether",
            ServerChatCommand::DestroyTethers => "destroy_tethers",
            ServerChatCommand::Mount => "mount",
            ServerChatCommand::Dismount => "dismount",
        }
    }

    /// The short keyword used to invoke the command, omitting the leading '/'.
    /// Returns None if the command doesn't have a short keyword
    pub fn short_keyword(&self) -> Option<&'static str> {
        Some(match self {
            ServerChatCommand::Faction => "f",
            ServerChatCommand::Group => "g",
            ServerChatCommand::Region => "r",
            ServerChatCommand::Say => "s",
            ServerChatCommand::Tell => "t",
            ServerChatCommand::World => "w",
            _ => return None,
        })
    }

    /// Produce an iterator over all the available commands
    pub fn iter() -> impl Iterator<Item = Self> + Clone { <Self as IntoEnumIterator>::iter() }

    /// A message that explains what the command does
    pub fn help_content(&self) -> Content {
        let data = self.data();

        let usage = std::iter::once(format!("/{}", self.keyword()))
            .chain(data.args.iter().map(|arg| arg.usage_string()))
            .collect::<Vec<_>>()
            .join(" ");

        Content::localized_with_args("command-help-template", [
            ("usage", Content::Plain(usage)),
            ("description", data.description),
        ])
    }

    /// Produce an iterator that first goes over all the short keywords
    /// and their associated commands and then iterates over all the normal
    /// keywords with their associated commands
    pub fn iter_with_keywords() -> impl Iterator<Item = (&'static str, Self)> {
        Self::iter()
        // Go through all the shortcuts first
        .filter_map(|c| c.short_keyword().map(|s| (s, c)))
        .chain(Self::iter().map(|c| (c.keyword(), c)))
    }

    pub fn needs_role(&self) -> Option<comp::AdminRole> { self.data().needs_role }

    /// Returns a format string for parsing arguments with scan_fmt
    pub fn arg_fmt(&self) -> String {
        self.data()
            .args
            .iter()
            .map(|arg| match arg {
                ArgumentSpec::PlayerName(_) => "{}",
                ArgumentSpec::EntityTarget(_) => "{}",
                ArgumentSpec::SiteName(_) => "{/.*/}",
                ArgumentSpec::Float(_, _, _) => "{}",
                ArgumentSpec::Integer(_, _, _) => "{d}",
                ArgumentSpec::Any(_, _) => "{}",
                ArgumentSpec::Command(_) => "{}",
                ArgumentSpec::Message(_) => "{/.*/}",
                ArgumentSpec::SubCommand => "{} {/.*/}",
                ArgumentSpec::Enum(_, _, _) => "{}",
                ArgumentSpec::AssetPath(_, _, _, _) => "{}",
                ArgumentSpec::Boolean(_, _, _) => "{}",
                ArgumentSpec::Flag(_) => "{}",
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Display for ServerChatCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.keyword())
    }
}

impl FromStr for ServerChatCommand {
    type Err = ();

    fn from_str(keyword: &str) -> Result<ServerChatCommand, ()> {
        Self::iter()
        // Go through all the shortcuts first
        .filter_map(|c| c.short_keyword().map(|s| (s, c)))
        .chain(Self::iter().map(|c| (c.keyword(), c)))
            // Find command with matching string as keyword
            .find_map(|(kwd, command)| (kwd == keyword).then_some(command))
            // Return error if not found
            .ok_or(())
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Requirement {
    Required,
    Optional,
}

/// Representation for chat command arguments
pub enum ArgumentSpec {
    /// The argument refers to a player by alias
    PlayerName(Requirement),
    /// The arguments refers to an entity in some way.
    EntityTarget(Requirement),
    // The argument refers to a site, by name.
    SiteName(Requirement),
    /// The argument is a float. The associated values are
    /// * label
    /// * suggested tab-completion
    /// * whether it's optional
    Float(&'static str, f32, Requirement),
    /// The argument is an integer. The associated values are
    /// * label
    /// * suggested tab-completion
    /// * whether it's optional
    Integer(&'static str, i32, Requirement),
    /// The argument is any string that doesn't contain spaces
    Any(&'static str, Requirement),
    /// The argument is a command name (such as in /help)
    Command(Requirement),
    /// This is the final argument, consuming all characters until the end of
    /// input.
    Message(Requirement),
    /// This command is followed by another command (such as in /sudo)
    SubCommand,
    /// The argument is likely an enum. The associated values are
    /// * label
    /// * Predefined string completions
    /// * whether it's optional
    Enum(&'static str, Vec<String>, Requirement),
    /// The argument is an asset path. The associated values are
    /// * label
    /// * Path prefix shared by all assets
    /// * List of all asset paths as strings for completion
    /// * whether it's optional
    AssetPath(&'static str, &'static str, Vec<String>, Requirement),
    /// The argument is likely a boolean. The associated values are
    /// * label
    /// * suggested tab-completion
    /// * whether it's optional
    Boolean(&'static str, String, Requirement),
    /// The argument is a flag that enables or disables a feature.
    Flag(&'static str),
}

impl ArgumentSpec {
    pub fn usage_string(&self) -> String {
        match self {
            ArgumentSpec::PlayerName(req) => {
                if &Requirement::Required == req {
                    "<player>".to_string()
                } else {
                    "[player]".to_string()
                }
            },
            ArgumentSpec::EntityTarget(req) => {
                if &Requirement::Required == req {
                    "<entity>".to_string()
                } else {
                    "[entity]".to_string()
                }
            },
            ArgumentSpec::SiteName(req) => {
                if &Requirement::Required == req {
                    "<site>".to_string()
                } else {
                    "[site]".to_string()
                }
            },
            ArgumentSpec::Float(label, _, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Integer(label, _, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Any(label, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Command(req) => {
                if &Requirement::Required == req {
                    "<[/]command>".to_string()
                } else {
                    "[[/]command]".to_string()
                }
            },
            ArgumentSpec::Message(req) => {
                if &Requirement::Required == req {
                    "<message>".to_string()
                } else {
                    "[message]".to_string()
                }
            },
            ArgumentSpec::SubCommand => "<[/]command> [args...]".to_string(),
            ArgumentSpec::Enum(label, _, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::AssetPath(label, _, _, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Boolean(label, _, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Flag(label) => {
                format!("[{}]", label)
            },
        }
    }

    pub fn requirement(&self) -> Requirement {
        match self {
            ArgumentSpec::PlayerName(r)
            | ArgumentSpec::EntityTarget(r)
            | ArgumentSpec::SiteName(r)
            | ArgumentSpec::Float(_, _, r)
            | ArgumentSpec::Integer(_, _, r)
            | ArgumentSpec::Any(_, r)
            | ArgumentSpec::Command(r)
            | ArgumentSpec::Message(r)
            | ArgumentSpec::Enum(_, _, r)
            | ArgumentSpec::AssetPath(_, _, _, r)
            | ArgumentSpec::Boolean(_, _, r) => *r,
            ArgumentSpec::Flag(_) => Requirement::Optional,
            ArgumentSpec::SubCommand => Requirement::Required,
        }
    }
}

pub trait CommandEnumArg: FromStr {
    fn all_options() -> Vec<String>;
}

macro_rules! impl_from_to_str_cmd {
    ($enum:ident, ($($attribute:ident => $str:expr),*)) => {
        impl std::str::FromStr for $enum {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        $str => Ok($enum::$attribute),
                    )*
                    s => Err(format!("Invalid variant: {s}")),
                }
            }
        }

        impl $crate::cmd::CommandEnumArg for $enum {
            fn all_options() -> Vec<String> {
                vec![$($str.to_string()),*]
            }
        }
    }
}

impl_from_to_str_cmd!(AuraKindVariant, (
    Buff => "buff",
    FriendlyFire => "friendly_fire",
    ForcePvP => "force_pvp"
));

impl_from_to_str_cmd!(GroupTarget, (
    InGroup => "in_group",
    OutOfGroup => "out_of_group",
    All => "all"
));

/// Parse a series of command arguments into values, including collecting all
/// trailing arguments.
#[macro_export]
macro_rules! parse_cmd_args {
    ($args:expr, $($t:ty),* $(, ..$tail:ty)? $(,)?) => {
        {
            let mut args = $args.into_iter().peekable();
            (
                // We only consume the input argument when parsing is successful. If this fails, we
                // will then attempt to parse it as the next argument type. This is done regardless
                // of whether the argument is optional because that information is not available
                // here. Nevertheless, if the caller only precedes to use the parsed arguments when
                // all required arguments parse successfully to `Some(val)` this should not create
                // any unexpected behavior.
                //
                // This does mean that optional arguments will be included in the trailing args or
                // that one optional arg could be interpreted as another, if the user makes a
                // mistake that causes an optional arg to fail to parse. But there is no way to
                // discern this in the current model with the optional args and trailing arg being
                // solely position based.
                $({
                    let parsed = args.peek().and_then(|s| s.parse::<$t>().ok());
                    // Consume successfully parsed arg.
                    if parsed.is_some() { args.next(); }
                    parsed
                }),*
                $(, args.map(|s| s.to_string()).collect::<$tail>())?
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comp::Item;

    #[test]
    fn verify_cmd_list_sorted() {
        let mut list = ServerChatCommand::iter()
            .map(|c| c.keyword())
            .collect::<Vec<_>>();

        // Vec::is_sorted is unstable, so we do it the hard way
        let list2 = list.clone();
        list.sort_unstable();
        assert_eq!(list, list2);
    }

    #[test]
    fn test_loading_skill_presets() {
        SkillPresetManifest::load_expect_combined_static(PRESET_MANIFEST_PATH);
    }

    #[test]
    fn test_load_kits() {
        let kits = KitManifest::load_expect_combined_static(KIT_MANIFEST_PATH).read();
        let mut rng = rand::thread_rng();
        for kit in kits.0.values() {
            for (item_id, _) in kit.iter() {
                match item_id {
                    KitSpec::Item(item_id) => {
                        Item::new_from_asset_expect(item_id);
                    },
                    KitSpec::ModularWeaponSet {
                        tool,
                        material,
                        hands,
                    } => {
                        comp::item::modular::generate_weapons(*tool, *material, *hands)
                            .unwrap_or_else(|_| {
                                panic!(
                                    "Failed to synthesize a modular {tool:?} set made of \
                                     {material:?}."
                                )
                            });
                    },
                    KitSpec::ModularWeaponRandom {
                        tool,
                        material,
                        hands,
                    } => {
                        comp::item::modular::random_weapon(*tool, *material, *hands, &mut rng)
                            .unwrap_or_else(|_| {
                                panic!(
                                    "Failed to synthesize a random {hands:?}-handed modular \
                                     {tool:?} made of {material:?}."
                                )
                            });
                    },
                }
            }
        }
    }
}
