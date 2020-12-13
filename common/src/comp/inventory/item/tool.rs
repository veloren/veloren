// Note: If you changes here "break" old character saves you can change the
// version in voxygen\src\meta.rs in order to reset save files to being empty

use crate::{
    assets::{self, Asset},
    comp::CharacterAbility,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{time::Duration};
use tracing::error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    Sword,
    Axe,
    Hammer,
    Bow,
    Dagger,
    Staff,
    Sceptre,
    Shield,
    Unique(UniqueKind),
    Debug,
    Farming,
    /// This is an placeholder item, it is used by non-humanoid npcs to attack
    Empty,
}

impl ToolKind {
    pub fn hands(&self) -> Hands {
        match self {
            ToolKind::Sword => Hands::TwoHand,
            ToolKind::Axe => Hands::TwoHand,
            ToolKind::Hammer => Hands::TwoHand,
            ToolKind::Bow => Hands::TwoHand,
            ToolKind::Dagger => Hands::OneHand,
            ToolKind::Staff => Hands::TwoHand,
            ToolKind::Sceptre => Hands::TwoHand,
            ToolKind::Shield => Hands::OneHand,
            ToolKind::Unique(_) => Hands::TwoHand,
            ToolKind::Debug => Hands::TwoHand,
            ToolKind::Farming => Hands::TwoHand,
            ToolKind::Empty => Hands::OneHand,
        }
    }
}

pub enum Hands {
    OneHand,
    TwoHand,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    equip_time_millis: u32,
    power: f32,
    speed: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub kind: ToolKind,
    pub stats: Stats,
    // TODO: item specific abilities
}

impl Tool {
    pub fn empty() -> Self {
        Self {
            kind: ToolKind::Empty,
            stats: Stats {
                equip_time_millis: 0,
                power: 1.00,
                speed: 1.00,
            },
        }
    }

    // Keep power between 0.5 and 2.00
    pub fn base_power(&self) -> f32 { self.stats.power }

    pub fn base_speed(&self) -> f32 { self.stats.speed }

    pub fn equip_time(&self) -> Duration {
        Duration::from_millis(self.stats.equip_time_millis as u64)
    }

    pub fn get_abilities(&self, map: &AbilityMap) -> AbilitySet<CharacterAbility> {
        if let Some(set) = map.0.get(&self.kind).cloned() {
            set.modified_by_tool(&self)
        } else {
            error!(
                "ToolKind: {:?} has no AbilitySet in the ability map falling back to default",
                &self.kind
            );
            Default::default()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilitySet<T> {
    pub primary: T,
    pub secondary: T,
    pub skills: Vec<T>,
}

impl AbilitySet<CharacterAbility> {
    fn modified_by_tool(self, tool: &Tool) -> Self {
        self.map(|a| a.adjusted_by_stats(tool.base_power(), tool.base_speed()))
    }
}

impl<T> AbilitySet<T> {
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> AbilitySet<U> {
        AbilitySet {
            primary: f(self.primary),
            secondary: f(self.secondary),
            skills: self.skills.into_iter().map(|x| f(x)).collect(),
        }
    }

    pub fn map_ref<U, F: FnMut(&T) -> U>(&self, mut f: F) -> AbilitySet<U> {
        AbilitySet {
            primary: f(&self.primary),
            secondary: f(&self.secondary),
            skills: self.skills.iter().map(|x| f(x)).collect(),
        }
    }
}

impl Default for AbilitySet<CharacterAbility> {
    fn default() -> Self {
        AbilitySet {
            primary: CharacterAbility::default(),
            secondary: CharacterAbility::default(),
            skills: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilityMap<T = CharacterAbility>(HashMap<ToolKind, AbilitySet<T>>);

impl Asset for AbilityMap<String> {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl assets::Compound for AbilityMap {
    fn load<S: assets_manager::source::Source>(
        cache: &assets_manager::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, assets::Error> {
        let manifest = cache.load::<AbilityMap<String>>(specifier)?.read();

        Ok(AbilityMap(
            manifest
                .0
                .iter()
                .map(|(kind, set)| {
                    (
                        kind.clone(),
                        // expect cannot fail because CharacterAbility always
                        // provides a default value in case of failure
                        set.map_ref(|s| cache.load_expect(&s).cloned()),
                    )
                })
                .collect(),
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UniqueKind {
    StoneGolemFist,
    BeastClaws,
    QuadMedQuick,
    QuadMedJump,
    QuadMedHoof,
    QuadMedBasic,
    QuadMedCharge,
    QuadLowRanged,
    QuadLowBreathe,
    QuadLowTail,
    QuadLowQuick,
    QuadLowBasic,
    QuadSmallBasic,
    TheropodBasic,
    TheropodBird,
}
