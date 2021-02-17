// Note: If you changes here "break" old character saves you can change the
// version in voxygen\src\meta.rs in order to reset save files to being empty

use crate::{
    assets::{self, Asset},
    comp::{item::ItemKind, skills::Skill, CharacterAbility, Item},
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;
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
    pub fn identifier_name(&self) -> &'static str {
        match self {
            ToolKind::Sword => "sword",
            ToolKind::Axe => "axe",
            ToolKind::Hammer => "hammer",
            ToolKind::Bow => "bow",
            ToolKind::Dagger => "dagger",
            ToolKind::Staff => "staff",
            ToolKind::Sceptre => "sceptre",
            ToolKind::Shield => "shield",
            ToolKind::Unique(_) => "unique",
            ToolKind::Debug => "debug",
            ToolKind::Farming => "farming",
            ToolKind::Empty => "empty",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Hands {
    One,
    Two,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub equip_time_millis: u32,
    pub power: f32,
    pub poise_strength: f32,
    pub speed: f32,
}

impl Stats {
    pub fn zeroed() -> Stats {
        Stats {
            equip_time_millis: 0,
            power: 0.0,
            poise_strength: 0.0,
            speed: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum StatKind {
    Direct(Stats),
    Modular,
}

impl StatKind {
    pub fn resolve_stats(&self, components: &[Item]) -> Stats {
        let mut stats = match self {
            StatKind::Direct(stats) => *stats,
            StatKind::Modular => Stats::zeroed(),
        };
        for item in components.iter() {
            if let ItemKind::ModularComponent(mc) = item.kind() {
                stats.equip_time_millis += mc.stats.equip_time_millis;
                stats.power += mc.stats.power;
                stats.poise_strength += mc.stats.poise_strength;
                stats.speed += mc.stats.speed;
            }
            // TODO: add stats from enhancement slots
        }
        // if an item has 0.0 speed, that panics due to being infinite duration, so
        // enforce speed >= 0.5
        stats.speed = stats.speed.max(0.5);
        stats
    }
}

impl From<(&[Item], &Tool)> for Stats {
    fn from((components, tool): (&[Item], &Tool)) -> Self {
        let raw_stats = tool.stats.resolve_stats(components);
        let (power, speed) = match tool.hands {
            Hands::One => (0.67, 1.33),
            // TODO: Restore this when one-handed weapons are made accessible
            // Hands::Two => (1.5, 0.75),
            Hands::Two => (1.0, 1.0),
        };
        Self {
            equip_time_millis: raw_stats.equip_time_millis,
            power: raw_stats.power * power,
            poise_strength: raw_stats.poise_strength,
            speed: raw_stats.speed * speed,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    pub kind: ToolKind,
    pub hands: Hands,
    pub stats: StatKind,
    // TODO: item specific abilities
}

impl Tool {
    // DO NOT USE UNLESS YOU KNOW WHAT YOU ARE DOING
    // Added for CSV import of stats
    pub fn new(
        kind: ToolKind,
        hands: Hands,
        equip_time_millis: u32,
        power: f32,
        poise_strength: f32,
        speed: f32,
    ) -> Self {
        Self {
            kind,
            hands,
            stats: StatKind::Direct(Stats {
                equip_time_millis,
                power,
                poise_strength,
                speed,
            }),
        }
    }

    pub fn empty() -> Self {
        Self {
            kind: ToolKind::Empty,
            hands: Hands::One,
            stats: StatKind::Direct(Stats {
                equip_time_millis: 0,
                power: 1.00,
                poise_strength: 1.00,
                speed: 1.00,
            }),
        }
    }

    // Keep power between 0.5 and 2.00
    pub fn base_power(&self, components: &[Item]) -> f32 {
        self.stats.resolve_stats(components).power
    }

    pub fn base_poise_strength(&self, components: &[Item]) -> f32 {
        self.stats.resolve_stats(components).poise_strength
    }

    pub fn base_speed(&self, components: &[Item]) -> f32 {
        self.stats.resolve_stats(components).speed
    }

    pub fn equip_time(&self, components: &[Item]) -> Duration {
        Duration::from_millis(self.stats.resolve_stats(components).equip_time_millis as u64)
    }

    pub fn get_abilities(
        &self,
        components: &[Item],
        map: &AbilityMap,
    ) -> AbilitySet<CharacterAbility> {
        if let Some(set) = map.0.get(&self.kind).cloned() {
            set.modified_by_tool(&self, components)
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
    pub abilities: Vec<(Option<Skill>, T)>,
}

impl AbilitySet<CharacterAbility> {
    pub fn modified_by_tool(self, tool: &Tool, components: &[Item]) -> Self {
        let stats = Stats::from((components, tool));
        self.map(|a| a.adjusted_by_stats(stats.power, stats.poise_strength, stats.speed))
    }
}

impl<T> AbilitySet<T> {
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> AbilitySet<U> {
        AbilitySet {
            primary: f(self.primary),
            secondary: f(self.secondary),
            abilities: self.abilities.into_iter().map(|(s, x)| (s, f(x))).collect(),
        }
    }

    pub fn map_ref<U, F: FnMut(&T) -> U>(&self, mut f: F) -> AbilitySet<U> {
        AbilitySet {
            primary: f(&self.primary),
            secondary: f(&self.secondary),
            abilities: self.abilities.iter().map(|(s, x)| (*s, f(x))).collect(),
        }
    }
}

impl Default for AbilitySet<CharacterAbility> {
    fn default() -> Self {
        AbilitySet {
            primary: CharacterAbility::default(),
            secondary: CharacterAbility::default(),
            abilities: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilityMap<T = CharacterAbility>(HashMap<ToolKind, AbilitySet<T>>);

impl Default for AbilityMap {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert(ToolKind::Empty, AbilitySet::default());
        AbilityMap(map)
    }
}

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
                        *kind,
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
