use serde::{Deserialize, Serialize};
use std::convert::AsRef;
use strum::{AsRefStr, EnumIter, EnumString};

/// Represents a key that the game recognises after input mapping.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize,
    AsRefStr,
    EnumIter,
    EnumString,
)]
pub enum GameInput {
    #[strum(serialize = "gameinput-primary")]
    Primary,
    #[strum(serialize = "gameinput-secondary")]
    Secondary,
    #[strum(serialize = "gameinput-block")]
    Block,
    #[strum(serialize = "gameinput-slot1")]
    Slot1,
    #[strum(serialize = "gameinput-slot2")]
    Slot2,
    #[strum(serialize = "gameinput-slot3")]
    Slot3,
    #[strum(serialize = "gameinput-slot4")]
    Slot4,
    #[strum(serialize = "gameinput-slot5")]
    Slot5,
    #[strum(serialize = "gameinput-slot6")]
    Slot6,
    #[strum(serialize = "gameinput-slot7")]
    Slot7,
    #[strum(serialize = "gameinput-slot8")]
    Slot8,
    #[strum(serialize = "gameinput-slot9")]
    Slot9,
    #[strum(serialize = "gameinput-slot10")]
    Slot10,
    #[strum(serialize = "gameinput-togglecursor")]
    ToggleCursor,
    #[strum(serialize = "gameinput-moveforward")]
    MoveForward,
    #[strum(serialize = "gameinput-moveback")]
    MoveBack,
    #[strum(serialize = "gameinput-moveleft")]
    MoveLeft,
    #[strum(serialize = "gameinput-moveright")]
    MoveRight,
    #[strum(serialize = "gameinput-jump")]
    Jump,
    #[strum(serialize = "gameinput-walljump")]
    WallJump,
    #[strum(serialize = "gameinput-sit")]
    Sit,
    #[strum(serialize = "gameinput-crawl")]
    Crawl,
    #[strum(serialize = "gameinput-dance")]
    Dance,
    #[strum(serialize = "gameinput-greet")]
    Greet,
    #[strum(serialize = "gameinput-glide")]
    Glide,
    #[strum(serialize = "gameinput-swimup")]
    SwimUp,
    #[strum(serialize = "gameinput-swimdown")]
    SwimDown,
    #[strum(serialize = "gameinput-fly")]
    Fly,
    #[strum(serialize = "gameinput-sneak")]
    Sneak,
    #[strum(serialize = "gameinput-cancelclimb")]
    CancelClimb,
    #[strum(serialize = "gameinput-togglelantern")]
    ToggleLantern,
    #[strum(serialize = "gameinput-mount")]
    Mount,
    #[strum(serialize = "gameinput-stayfollow")]
    StayFollow,
    #[strum(serialize = "gameinput-chat")]
    Chat,
    #[strum(serialize = "gameinput-command")]
    Command,
    #[strum(serialize = "gameinput-escape")]
    Escape,
    #[strum(serialize = "gameinput-map")]
    Map,
    #[strum(serialize = "gameinput-inventory")]
    Inventory,
    #[strum(serialize = "gameinput-trade")]
    Trade,
    #[strum(serialize = "gameinput-social")]
    Social,
    #[strum(serialize = "gameinput-crafting")]
    Crafting,
    #[strum(serialize = "gameinput-diary")]
    Diary,
    #[strum(serialize = "gameinput-settings")]
    Settings,
    #[strum(serialize = "gameinput-controls")]
    Controls,
    #[strum(serialize = "gameinput-toggleinterface")]
    ToggleInterface,
    #[strum(serialize = "gameinput-toggledebug")]
    ToggleDebug,
    #[cfg(feature = "egui-ui")]
    #[strum(serialize = "gameinput-toggle_egui_debug")]
    ToggleEguiDebug,
    #[strum(serialize = "gameinput-togglechat")]
    ToggleChat,
    #[strum(serialize = "gameinput-fullscreen")]
    Fullscreen,
    #[strum(serialize = "gameinput-screenshot")]
    Screenshot,
    #[strum(serialize = "gameinput-toggleingameui")]
    ToggleIngameUi,
    #[strum(serialize = "gameinput-roll")]
    Roll,
    #[strum(serialize = "gameinput-giveup")]
    GiveUp,
    #[strum(serialize = "gameinput-respawn")]
    Respawn,
    #[strum(serialize = "gameinput-interact")]
    Interact,
    #[strum(serialize = "gameinput-togglewield")]
    ToggleWield,
    #[strum(serialize = "gameinput-swaploadout")]
    SwapLoadout,
    #[strum(serialize = "gameinput-freelook")]
    FreeLook,
    #[strum(serialize = "gameinput-autowalk")]
    AutoWalk,
    #[strum(serialize = "gameinput-zoomin")]
    ZoomIn,
    #[strum(serialize = "gameinput-zoomout")]
    ZoomOut,
    #[strum(serialize = "gameinput-zoomlock")]
    ZoomLock,
    #[strum(serialize = "gameinput-cameraclamp")]
    CameraClamp,
    #[strum(serialize = "gameinput-cyclecamera")]
    CycleCamera,
    #[strum(serialize = "gameinput-select")]
    Select,
    #[strum(serialize = "gameinput-acceptgroupinvite")]
    AcceptGroupInvite,
    #[strum(serialize = "gameinput-declinegroupinvite")]
    DeclineGroupInvite,
    #[strum(serialize = "gameinput-mapzoomin")]
    MapZoomIn,
    #[strum(serialize = "gameinput-mapzoomout")]
    MapZoomOut,
    #[strum(serialize = "gameinput-map-locationmarkerbutton")]
    MapSetMarker,
    #[strum(serialize = "gameinput-spectatespeedboost")]
    SpectateSpeedBoost,
    #[strum(serialize = "gameinput-spectateviewpoint")]
    SpectateViewpoint,
    #[strum(serialize = "gameinput-mutemaster")]
    MuteMaster,
    #[strum(serialize = "gameinput-muteinactivemaster")]
    MuteInactiveMaster,
    #[strum(serialize = "gameinput-mutemusic")]
    MuteMusic,
    #[strum(serialize = "gameinput-mutesfx")]
    MuteSfx,
    #[strum(serialize = "gameinput-muteambience")]
    MuteAmbience,
    #[strum(serialize = "gameinput-togglewalk")]
    ToggleWalk,
}

impl GameInput {
    pub fn get_localization_key(&self) -> &str { self.as_ref() }

    /// Return true if `a` and `b` are able to be bound to the same key at the
    /// same time without conflict. For example, the player can't jump and climb
    /// at the same time, so these can be bound to the same key.
    pub fn can_share_bindings(a: GameInput, b: GameInput) -> bool {
        let bindings_a = a.get_representative_bindings();
        let bindings_b = b.get_representative_bindings();

        if bindings_a.is_empty() && bindings_b.is_empty() {
            return a == b;
        }
        if bindings_a.is_empty() {
            return bindings_b.contains(&a);
        }
        if bindings_b.is_empty() {
            return bindings_a.contains(&b);
        }

        bindings_a.iter().any(|x| bindings_b.contains(x))
    }

    /// If two GameInputs are able to be bound at the same time, then they will
    /// return a slice that contains the other GameInput.
    /// Since a GameInput might be fine to be shared with multiple other
    /// GameInputs a slice is required instead of just a single GameInput.
    /// This models the Find operation of a disjoint-set data
    /// structure.
    fn get_representative_bindings(self) -> &'static [GameInput] {
        match self {
            GameInput::SwimUp | GameInput::Respawn | GameInput::GiveUp => &[GameInput::Jump],

            GameInput::AutoWalk | GameInput::FreeLook => &[GameInput::FreeLook],

            GameInput::SpectateSpeedBoost => &[GameInput::Glide],
            GameInput::WallJump => &[GameInput::Mount, GameInput::Jump],

            GameInput::SwimDown | GameInput::Sneak | GameInput::CancelClimb => &[GameInput::Roll],

            GameInput::SpectateViewpoint => &[GameInput::MapSetMarker],

            _ => &[],
        }
    }
}
