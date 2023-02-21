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
    #[strum(serialize = "gameinput-sit")]
    Sit,
    #[strum(serialize = "gameinput-dance")]
    Dance,
    #[strum(serialize = "gameinput-greet")]
    Greet,
    #[strum(serialize = "gameinput-glide")]
    Glide,
    #[strum(serialize = "gameinput-climb")]
    Climb,
    #[strum(serialize = "gameinput-climbdown")]
    ClimbDown,
    #[strum(serialize = "gameinput-swimup")]
    SwimUp,
    #[strum(serialize = "gameinput-swimdown")]
    SwimDown,
    #[strum(serialize = "gameinput-fly")]
    Fly,
    #[strum(serialize = "gameinput-sneak")]
    Sneak,
    #[strum(serialize = "gameinput-togglelantern")]
    ToggleLantern,
    #[strum(serialize = "gameinput-mount")]
    Mount,
    #[strum(serialize = "gameinput-chat")]
    Chat,
    #[strum(serialize = "gameinput-command")]
    Command,
    #[strum(serialize = "gameinput-escape")]
    Escape,
    #[strum(serialize = "gameinput-map")]
    Map,
    #[strum(serialize = "gameinput-bag")]
    Bag,
    #[strum(serialize = "gameinput-trade")]
    Trade,
    #[strum(serialize = "gameinput-social")]
    Social,
    #[strum(serialize = "gameinput-crafting")]
    Crafting,
    #[strum(serialize = "gameinput-spellbook")]
    Spellbook,
    #[strum(serialize = "gameinput-settings")]
    Settings,
    #[strum(serialize = "gameinput-toggleinterface")]
    ToggleInterface,
    #[strum(serialize = "gameinput-help")]
    Help,
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
}

impl GameInput {
    pub fn get_localization_key(&self) -> &str { self.as_ref() }

    /// Return true if `a` and `b` are able to be bound to the same key at the
    /// same time without conflict. For example, the player can't jump and climb
    /// at the same time, so these can be bound to the same key.
    pub fn can_share_bindings(a: GameInput, b: GameInput) -> bool {
        a.get_representative_binding() == b.get_representative_binding()
    }

    /// If two GameInputs are able to be bound at the same time, then they will
    /// return the same value from this function (the representative value for
    /// that set). This models the Find operation of a disjoint-set data
    /// structure.
    fn get_representative_binding(&self) -> GameInput {
        match self {
            GameInput::Jump => GameInput::Jump,
            GameInput::Climb => GameInput::Jump,
            GameInput::SwimUp => GameInput::Jump,
            GameInput::Respawn => GameInput::Jump,

            GameInput::FreeLook => GameInput::FreeLook,
            GameInput::AutoWalk => GameInput::FreeLook,

            _ => *self,
        }
    }
}
