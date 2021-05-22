use crate::{
    assets,
    comp::{self, buff::BuffKind, AdminRole as Role, Skill},
    npc, terrain,
};
use assets::AssetExt;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    path::Path,
    str::FromStr,
};
use strum::IntoEnumIterator;
use tracing::warn;

/// Struct representing a command that a user can run from server chat.
pub struct ChatCommandData {
    /// A list of arguments useful for both tab completion and parsing
    pub args: Vec<ArgumentSpec>,
    /// A one-line message that explains what the command does
    pub description: &'static str,
    /// Whether the command requires administrator permissions.
    pub needs_role: Option<Role>,
}

impl ChatCommandData {
    pub fn new(
        args: Vec<ArgumentSpec>,
        description: &'static str,
        needs_role: Option<Role>,
    ) -> Self {
        Self {
            args,
            description,
            needs_role,
        }
    }
}

// Please keep this sorted alphabetically :-)
#[derive(Copy, Clone)]
pub enum ChatCommand {
    Adminify,
    Airship,
    Alias,
    ApplyBuff,
    Ban,
    Build,
    BuildAreaAdd,
    BuildAreaList,
    BuildAreaRemove,
    Campfire,
    DebugColumn,
    DisconnectAllPlayers,
    DropAll,
    Dummy,
    Explosion,
    Faction,
    GiveItem,
    Goto,
    Group,
    GroupInvite,
    GroupKick,
    GroupLeave,
    GroupPromote,
    Health,
    Help,
    Home,
    JoinFaction,
    Jump,
    Kick,
    Kill,
    KillNpcs,
    Kit,
    Lantern,
    Light,
    MakeBlock,
    MakeSprite,
    Motd,
    Object,
    PermitBuild,
    Players,
    Region,
    RemoveLights,
    RevokeBuild,
    RevokeBuildAll,
    Safezone,
    Say,
    ServerPhysics,
    SetMotd,
    Site,
    SkillPoint,
    SkillPreset,
    Spawn,
    Sudo,
    Tell,
    Time,
    Tp,
    Unban,
    Version,
    Waypoint,
    Whitelist,
    Wiring,
    World,
}

// Thank you for keeping this sorted alphabetically :-)
pub static CHAT_COMMANDS: &[ChatCommand] = &[
    ChatCommand::Adminify,
    ChatCommand::Airship,
    ChatCommand::Alias,
    ChatCommand::ApplyBuff,
    ChatCommand::Ban,
    ChatCommand::Build,
    ChatCommand::BuildAreaAdd,
    ChatCommand::BuildAreaList,
    ChatCommand::BuildAreaRemove,
    ChatCommand::Campfire,
    ChatCommand::DebugColumn,
    ChatCommand::DisconnectAllPlayers,
    ChatCommand::DropAll,
    ChatCommand::Dummy,
    ChatCommand::Explosion,
    ChatCommand::Faction,
    ChatCommand::GiveItem,
    ChatCommand::Goto,
    ChatCommand::Group,
    ChatCommand::GroupInvite,
    ChatCommand::GroupKick,
    ChatCommand::GroupLeave,
    ChatCommand::GroupPromote,
    ChatCommand::Health,
    ChatCommand::Help,
    ChatCommand::Home,
    ChatCommand::JoinFaction,
    ChatCommand::Jump,
    ChatCommand::Kick,
    ChatCommand::Kill,
    ChatCommand::KillNpcs,
    ChatCommand::Kit,
    ChatCommand::Lantern,
    ChatCommand::Light,
    ChatCommand::MakeBlock,
    ChatCommand::MakeSprite,
    ChatCommand::Motd,
    ChatCommand::Object,
    ChatCommand::PermitBuild,
    ChatCommand::Players,
    ChatCommand::Region,
    ChatCommand::RemoveLights,
    ChatCommand::RevokeBuild,
    ChatCommand::RevokeBuildAll,
    ChatCommand::Safezone,
    ChatCommand::Say,
    ChatCommand::ServerPhysics,
    ChatCommand::SetMotd,
    ChatCommand::Site,
    ChatCommand::SkillPoint,
    ChatCommand::SkillPreset,
    ChatCommand::Spawn,
    ChatCommand::Sudo,
    ChatCommand::Tell,
    ChatCommand::Time,
    ChatCommand::Tp,
    ChatCommand::Unban,
    ChatCommand::Version,
    ChatCommand::Waypoint,
    ChatCommand::Whitelist,
    ChatCommand::Wiring,
    ChatCommand::World,
];

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct KitManifest(pub HashMap<String, Vec<(String, u32)>>);
impl assets::Asset for KitManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct SkillPresetManifest(pub HashMap<String, Vec<(Skill, u8)>>);
impl assets::Asset for SkillPresetManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

lazy_static! {
    pub static ref CHAT_SHORTCUTS: HashMap<char, ChatCommand> = [
        ('f', ChatCommand::Faction),
        ('g', ChatCommand::Group),
        ('r', ChatCommand::Region),
        ('s', ChatCommand::Say),
        ('t', ChatCommand::Tell),
        ('w', ChatCommand::World),
    ].iter().cloned().collect();

    static ref ALIGNMENTS: Vec<String> = vec!["wild", "enemy", "npc", "pet"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    static ref SKILL_TREES: Vec<String> = vec!["general", "sword", "axe", "hammer", "bow", "staff", "sceptre"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    /// TODO: Make this use hot-reloading
    static ref ENTITIES: Vec<String> = {
        let npc_names = &*npc::NPC_NAMES.read();
        let mut souls = Vec::new();
        macro_rules! push_souls {
            ($species:tt) => {
                for s in comp::$species::ALL_SPECIES.iter() {
                    souls.push(npc_names.$species.species[s].keyword.clone())
                }
            };
            ($base:tt, $($species:tt),+ $(,)?) => {
                push_souls!($base);
                push_souls!($($species),+);
            }
        }
        for npc in npc::ALL_NPCS.iter() {
            souls.push(npc_names[*npc].keyword.clone())
        }

        // See `[AllBodies](crate::comp::body::AllBodies)`
        push_souls!(
            humanoid,
            quadruped_small,
            quadruped_medium,
            quadruped_low,
            bird_medium,
            bird_large,
            fish_small,
            fish_medium,
            biped_small,
            biped_large,
            theropod,
            dragon,
            golem,
        );

        souls
    };
    static ref OBJECTS: Vec<String> = comp::object::ALL_OBJECTS
        .iter()
        .map(|o| o.to_string().to_string())
        .collect();
    static ref TIMES: Vec<String> = vec![
        "midnight", "night", "dawn", "morning", "day", "noon", "dusk"
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    pub static ref BUFF_PARSER: HashMap<String, BuffKind> = {
        let string_from_buff = |kind| match kind {
            BuffKind::Burning => "burning",
            BuffKind::Regeneration => "regeration",
            BuffKind::Saturation => "saturation",
            BuffKind::Bleeding => "bleeding",
            BuffKind::Cursed => "cursed",
            BuffKind::Potion => "potion",
            BuffKind::CampfireHeal => "campfire_heal",
            BuffKind::IncreaseMaxEnergy => "increase_max_energy",
            BuffKind::IncreaseMaxHealth => "increase_max_health",
            BuffKind::Invulnerability => "invulnerability",
            BuffKind::ProtectingWard => "protecting_ward",
            BuffKind::Frenzied => "frenzied",
            BuffKind::Crippled => "crippled",
        };
        let mut buff_parser = HashMap::new();
        BuffKind::iter().for_each(|kind| {buff_parser.insert(string_from_buff(kind).to_string(), kind);});
        buff_parser
    };

    pub static ref BUFF_PACK: Vec<String> = {
        let mut buff_pack: Vec<_> = BUFF_PARSER.keys().cloned().collect();
        // Remove invulnerability as it removes debuffs
        buff_pack.retain(|kind| kind != "invulnerability");
        buff_pack
    };

    static ref BUFFS: Vec<String> = {
        let mut buff_pack: Vec<_> = BUFF_PARSER.keys().cloned().collect();
        // Add all as valid command
        buff_pack.push("all".to_string());
        buff_pack
    };

    static ref BLOCK_KINDS: Vec<String> = terrain::block::BLOCK_KINDS
        .keys()
        .cloned()
        .collect();

    static ref SPRITE_KINDS: Vec<String> = terrain::sprite::SPRITE_KINDS
        .keys()
        .cloned()
        .collect();

    static ref ROLES: Vec<String> = ["admin", "moderator"].iter().copied().map(Into::into).collect();

    /// List of item specifiers. Useful for tab completing
    static ref ITEM_SPECS: Vec<String> = {
        let path = assets::ASSETS_PATH.join("common").join("items");
        let mut items = vec![];
        fn list_items (path: &Path, base: &Path, mut items: &mut Vec<String>) -> std::io::Result<()>{
            for entry in std::fs::read_dir(path)? {
                let path = entry?.path();
                if path.is_dir(){
                    list_items(&path, &base, &mut items)?;
                } else if let Ok(path) = path.strip_prefix(base) {
                    let path = path.to_string_lossy().trim_end_matches(".ron").replace('/', ".");
                    items.push(path);
                }
            }
            Ok(())
        }
        if list_items(&path, &assets::ASSETS_PATH, &mut items).is_err() {
            warn!("There was a problem listing item assets");
        }
        items.sort();
        items
    };

    static ref KITS: Vec<String> = {
        if let Ok(kits) = KitManifest::load("server.manifests.kits") {
            kits.read().0.keys().cloned().collect()
        } else {
            Vec::new()
        }
    };

    static ref PRESETS: HashMap<String, Vec<(Skill, u8)>> = {
        if let Ok(presets) = SkillPresetManifest::load("server.manifests.presets") {
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
}

impl ChatCommand {
    pub fn data(&self) -> ChatCommandData {
        use ArgumentSpec::*;
        use Requirement::*;
        use Role::*;
        let cmd = ChatCommandData::new;
        match self {
            ChatCommand::Adminify => cmd(
                vec![PlayerName(Required), Enum("role", ROLES.clone(), Optional)],
                "Temporarily gives a player a restricted admin role or removes the current one \
                 (if not given)",
                Some(Admin),
            ),
            ChatCommand::Airship => cmd(
                vec![Float("destination_degrees_ccw_of_east", 90.0, Optional)],
                "Spawns an airship",
                Some(Admin),
            ),
            ChatCommand::Alias => cmd(vec![Any("name", Required)], "Change your alias", None),
            ChatCommand::ApplyBuff => cmd(
                vec![
                    Enum("buff", BUFFS.clone(), Required),
                    Float("strength", 0.01, Optional),
                    Float("duration", 10.0, Optional),
                ],
                "Cast a buff on player",
                Some(Admin),
            ),
            ChatCommand::Ban => cmd(
                vec![
                    Any("username", Required),
                    Boolean("overwrite", "true".to_string(), Optional),
                    Any("ban duration", Optional),
                    Message(Optional),
                ],
                "Ban a player with a given username, for a given duration (if provided).  Pass \
                 true for overwrite to alter an existing ban..",
                Some(Moderator),
            ),
            ChatCommand::Build => cmd(vec![], "Toggles build mode on and off", None),
            ChatCommand::BuildAreaAdd => cmd(
                vec![
                    Any("name", Required),
                    Integer("xlo", 0, Required),
                    Integer("xhi", 10, Required),
                    Integer("ylo", 0, Required),
                    Integer("yhi", 10, Required),
                    Integer("zlo", 0, Required),
                    Integer("zhi", 10, Required),
                ],
                "Adds a new build area",
                Some(Admin),
            ),
            ChatCommand::BuildAreaList => cmd(vec![], "List all build areas", Some(Admin)),
            ChatCommand::BuildAreaRemove => cmd(
                vec![Any("name", Required)],
                "Removes specified build area",
                Some(Admin),
            ),
            ChatCommand::Campfire => cmd(vec![], "Spawns a campfire", Some(Admin)),
            ChatCommand::DebugColumn => cmd(
                vec![Integer("x", 15000, Required), Integer("y", 15000, Required)],
                "Prints some debug information about a column",
                Some(Moderator),
            ),
            ChatCommand::DisconnectAllPlayers => cmd(
                vec![Any("confirm", Required)],
                "Disconnects all players from the server",
                Some(Admin),
            ),
            ChatCommand::DropAll => cmd(
                vec![],
                "Drops all your items on the ground",
                Some(Moderator),
            ),
            ChatCommand::Dummy => cmd(vec![], "Spawns a training dummy", Some(Admin)),
            ChatCommand::Explosion => cmd(
                vec![Float("radius", 5.0, Required)],
                "Explodes the ground around you",
                Some(Admin),
            ),
            ChatCommand::Faction => cmd(
                vec![Message(Optional)],
                "Send messages to your faction",
                None,
            ),
            ChatCommand::GiveItem => cmd(
                vec![
                    Enum("item", ITEM_SPECS.clone(), Required),
                    Integer("num", 1, Optional),
                ],
                "Give yourself some items",
                Some(Admin),
            ),
            ChatCommand::Goto => cmd(
                vec![
                    Float("x", 0.0, Required),
                    Float("y", 0.0, Required),
                    Float("z", 0.0, Required),
                ],
                "Teleport to a position",
                Some(Admin),
            ),
            ChatCommand::Group => cmd(vec![Message(Optional)], "Send messages to your group", None),
            ChatCommand::GroupInvite => cmd(
                vec![PlayerName(Required)],
                "Invite a player to join a group",
                None,
            ),
            ChatCommand::GroupKick => cmd(
                vec![PlayerName(Required)],
                "Remove a player from a group",
                None,
            ),
            ChatCommand::GroupLeave => cmd(vec![], "Leave the current group", None),
            ChatCommand::GroupPromote => cmd(
                vec![PlayerName(Required)],
                "Promote a player to group leader",
                None,
            ),
            ChatCommand::Health => cmd(
                vec![Integer("hp", 100, Required)],
                "Set your current health",
                Some(Admin),
            ),
            ChatCommand::Help => ChatCommandData::new(
                vec![Command(Optional)],
                "Display information about commands",
                None,
            ),
            ChatCommand::Home => cmd(vec![], "Return to the home town", None),
            ChatCommand::JoinFaction => ChatCommandData::new(
                vec![Any("faction", Optional)],
                "Join/leave the specified faction",
                None,
            ),
            ChatCommand::Jump => cmd(
                vec![
                    Float("x", 0.0, Required),
                    Float("y", 0.0, Required),
                    Float("z", 0.0, Required),
                ],
                "Offset your current position",
                Some(Admin),
            ),
            ChatCommand::Kick => cmd(
                vec![Any("username", Required), Message(Optional)],
                "Kick a player with a given username",
                Some(Moderator),
            ),
            ChatCommand::Kill => cmd(vec![], "Kill yourself", None),
            ChatCommand::KillNpcs => cmd(vec![], "Kill the NPCs", Some(Admin)),
            ChatCommand::Kit => cmd(
                vec![Enum("kit_name", KITS.to_vec(), Required)],
                "Place a set of items into your inventory.",
                Some(Admin),
            ),
            ChatCommand::Lantern => cmd(
                vec![
                    Float("strength", 5.0, Required),
                    Float("r", 1.0, Optional),
                    Float("g", 1.0, Optional),
                    Float("b", 1.0, Optional),
                ],
                "Change your lantern's strength and color",
                Some(Admin),
            ),
            ChatCommand::Light => cmd(
                vec![
                    Float("r", 1.0, Optional),
                    Float("g", 1.0, Optional),
                    Float("b", 1.0, Optional),
                    Float("x", 0.0, Optional),
                    Float("y", 0.0, Optional),
                    Float("z", 0.0, Optional),
                    Float("strength", 5.0, Optional),
                ],
                "Spawn entity with light",
                Some(Admin),
            ),
            ChatCommand::MakeBlock => cmd(
                vec![Enum("block", BLOCK_KINDS.clone(), Required)],
                "Make a block at your location",
                Some(Admin),
            ),
            ChatCommand::MakeSprite => cmd(
                vec![Enum("sprite", SPRITE_KINDS.clone(), Required)],
                "Make a sprite at your location",
                Some(Admin),
            ),
            ChatCommand::Motd => cmd(vec![Message(Optional)], "View the server description", None),
            ChatCommand::Object => cmd(
                vec![Enum("object", OBJECTS.clone(), Required)],
                "Spawn an object",
                Some(Admin),
            ),
            ChatCommand::PermitBuild => cmd(
                vec![Any("area_name", Required)],
                "Grants player a bounded box they can build in",
                Some(Admin),
            ),
            ChatCommand::Players => cmd(vec![], "Lists players currently online", None),
            ChatCommand::RemoveLights => cmd(
                vec![Float("radius", 20.0, Optional)],
                "Removes all lights spawned by players",
                Some(Admin),
            ),
            ChatCommand::RevokeBuild => cmd(
                vec![Any("area_name", Required)],
                "Revokes build area permission for player",
                Some(Admin),
            ),
            ChatCommand::RevokeBuildAll => cmd(
                vec![],
                "Revokes all build area permissions for player",
                Some(Admin),
            ),
            ChatCommand::Region => cmd(
                vec![Message(Optional)],
                "Send messages to everyone in your region of the world",
                None,
            ),
            ChatCommand::Safezone => cmd(
                vec![Float("range", 100.0, Optional)],
                "Creates a safezone",
                Some(Moderator),
            ),
            ChatCommand::Say => cmd(
                vec![Message(Optional)],
                "Send messages to everyone within shouting distance",
                None,
            ),
            ChatCommand::ServerPhysics => cmd(
                vec![
                    Any("username", Required),
                    Boolean("enabled", "true".to_string(), Optional),
                ],
                "Set/unset server-authoritative physics for an account",
                Some(Moderator),
            ),
            ChatCommand::SetMotd => cmd(
                vec![Message(Optional)],
                "Set the server description",
                Some(Admin),
            ),
            // Uses Message because site names can contain spaces, which would be assumed to be
            // separators otherwise
            ChatCommand::Site => cmd(
                vec![Message(Required)],
                "Teleport to a site",
                Some(Moderator),
            ),
            ChatCommand::SkillPoint => cmd(
                vec![
                    Enum("skill tree", SKILL_TREES.clone(), Required),
                    Integer("amount", 1, Optional),
                ],
                "Give yourself skill points for a particular skill tree",
                Some(Admin),
            ),
            ChatCommand::SkillPreset => cmd(
                vec![Enum("preset_name", PRESET_LIST.to_vec(), Required)],
                "Gives your character desired skills.",
                Some(Admin),
            ),
            ChatCommand::Spawn => cmd(
                vec![
                    Enum("alignment", ALIGNMENTS.clone(), Required),
                    Enum("entity", ENTITIES.clone(), Required),
                    Integer("amount", 1, Optional),
                    Boolean("ai", "true".to_string(), Optional),
                ],
                "Spawn a test entity",
                Some(Admin),
            ),
            ChatCommand::Sudo => cmd(
                vec![PlayerName(Required), SubCommand],
                "Run command as if you were another player",
                Some(Moderator),
            ),
            ChatCommand::Tell => cmd(
                vec![PlayerName(Required), Message(Optional)],
                "Send a message to another player",
                None,
            ),
            ChatCommand::Time => cmd(
                vec![Enum("time", TIMES.clone(), Optional)],
                "Set the time of day",
                Some(Admin),
            ),
            ChatCommand::Tp => cmd(
                vec![PlayerName(Optional)],
                "Teleport to another player",
                Some(Moderator),
            ),
            ChatCommand::Unban => cmd(
                vec![Any("username", Required)],
                "Remove the ban for the given username",
                Some(Moderator),
            ),
            ChatCommand::Version => cmd(vec![], "Prints server version", None),
            ChatCommand::Waypoint => cmd(
                vec![],
                "Set your waypoint to your current position",
                Some(Admin),
            ),
            ChatCommand::Wiring => cmd(vec![], "Create wiring element", Some(Admin)),
            ChatCommand::Whitelist => cmd(
                vec![Any("add/remove", Required), Any("username", Required)],
                "Adds/removes username to whitelist",
                Some(Moderator),
            ),
            ChatCommand::World => cmd(
                vec![Message(Optional)],
                "Send messages to everyone on the server",
                None,
            ),
        }
    }

    /// The keyword used to invoke the command, omitting the leading '/'.
    pub fn keyword(&self) -> &'static str {
        match self {
            ChatCommand::Adminify => "adminify",
            ChatCommand::Airship => "airship",
            ChatCommand::Alias => "alias",
            ChatCommand::ApplyBuff => "buff",
            ChatCommand::Ban => "ban",
            ChatCommand::Build => "build",
            ChatCommand::BuildAreaAdd => "build_area_add",
            ChatCommand::BuildAreaList => "build_area_list",
            ChatCommand::BuildAreaRemove => "build_area_remove",
            ChatCommand::Campfire => "campfire",
            ChatCommand::DebugColumn => "debug_column",
            ChatCommand::DisconnectAllPlayers => "disconnect_all_players",
            ChatCommand::DropAll => "dropall",
            ChatCommand::Dummy => "dummy",
            ChatCommand::Explosion => "explosion",
            ChatCommand::Faction => "faction",
            ChatCommand::GiveItem => "give_item",
            ChatCommand::Goto => "goto",
            ChatCommand::Group => "group",
            ChatCommand::GroupInvite => "group_invite",
            ChatCommand::GroupKick => "group_kick",
            ChatCommand::GroupPromote => "group_promote",
            ChatCommand::GroupLeave => "group_leave",
            ChatCommand::Health => "health",
            ChatCommand::JoinFaction => "join_faction",
            ChatCommand::Help => "help",
            ChatCommand::Home => "home",
            ChatCommand::Jump => "jump",
            ChatCommand::Kick => "kick",
            ChatCommand::Kill => "kill",
            ChatCommand::Kit => "kit",
            ChatCommand::KillNpcs => "kill_npcs",
            ChatCommand::Lantern => "lantern",
            ChatCommand::Light => "light",
            ChatCommand::MakeBlock => "make_block",
            ChatCommand::MakeSprite => "make_sprite",
            ChatCommand::Motd => "motd",
            ChatCommand::Object => "object",
            ChatCommand::PermitBuild => "permit_build",
            ChatCommand::Players => "players",
            ChatCommand::Region => "region",
            ChatCommand::RemoveLights => "remove_lights",
            ChatCommand::RevokeBuild => "revoke_build",
            ChatCommand::RevokeBuildAll => "revoke_build_all",
            ChatCommand::Safezone => "safezone",
            ChatCommand::Say => "say",
            ChatCommand::ServerPhysics => "server_physics",
            ChatCommand::SetMotd => "set_motd",
            ChatCommand::Site => "site",
            ChatCommand::SkillPoint => "skill_point",
            ChatCommand::SkillPreset => "skill_preset",
            ChatCommand::Spawn => "spawn",
            ChatCommand::Sudo => "sudo",
            ChatCommand::Tell => "tell",
            ChatCommand::Time => "time",
            ChatCommand::Tp => "tp",
            ChatCommand::Unban => "unban",
            ChatCommand::Version => "version",
            ChatCommand::Waypoint => "waypoint",
            ChatCommand::Wiring => "wiring",
            ChatCommand::Whitelist => "whitelist",
            ChatCommand::World => "world",
        }
    }

    /// A message that explains what the command does
    pub fn help_string(&self) -> String {
        let data = self.data();
        let usage = std::iter::once(format!("/{}", self.keyword()))
            .chain(data.args.iter().map(|arg| arg.usage_string()))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{}: {}", usage, data.description)
    }

    /// A boolean that is used to check whether the command requires
    /// administrator permissions or not.
    pub fn needs_role(&self) -> Option<Role> { self.data().needs_role }

    /// Returns a format string for parsing arguments with scan_fmt
    pub fn arg_fmt(&self) -> String {
        self.data()
            .args
            .iter()
            .map(|arg| match arg {
                ArgumentSpec::PlayerName(_) => "{}",
                ArgumentSpec::Float(_, _, _) => "{}",
                ArgumentSpec::Integer(_, _, _) => "{d}",
                ArgumentSpec::Any(_, _) => "{}",
                ArgumentSpec::Command(_) => "{}",
                ArgumentSpec::Message(_) => "{/.*/}",
                ArgumentSpec::SubCommand => "{} {/.*/}",
                ArgumentSpec::Enum(_, _, _) => "{}",
                ArgumentSpec::Boolean(_, _, _) => "{}",
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Display for ChatCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.keyword())
    }
}

impl FromStr for ChatCommand {
    type Err = ();

    fn from_str(keyword: &str) -> Result<ChatCommand, ()> {
        let kwd = if let Some(stripped) = keyword.strip_prefix('/') {
            stripped
        } else {
            &keyword
        };
        if keyword.len() == 1 {
            if let Some(c) = keyword
                .chars()
                .next()
                .as_ref()
                .and_then(|k| CHAT_SHORTCUTS.get(k))
            {
                return Ok(*c);
            }
        } else {
            for c in CHAT_COMMANDS {
                if kwd == c.keyword() {
                    return Ok(*c);
                }
            }
        }
        Err(())
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum Requirement {
    Required,
    Optional,
}

/// Representation for chat command arguments
pub enum ArgumentSpec {
    /// The argument refers to a player by alias
    PlayerName(Requirement),
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
    /// The argument is likely a boolean. The associated values are
    /// * label
    /// * suggested tab-completion
    /// * whether it's optional
    Boolean(&'static str, String, Requirement),
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
                    format! {"<{}>", label}
                } else {
                    format! {"[{}]", label}
                }
            },
            ArgumentSpec::Boolean(label, _, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_skill_presets() {
        SkillPresetManifest::load_expect("server.manifests.presets");
    }
}
