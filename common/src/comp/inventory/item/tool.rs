// Note: If you changes here "break" old character saves you can change the
// version in voxygen\src\meta.rs in order to reset save files to being empty

use crate::{
    assets::{self, Asset, AssetExt},
    comp::{skills::Skill, CharacterAbility},
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{
    ops::{AddAssign, DivAssign, Mul, MulAssign, Sub},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub enum ToolKind {
    // weapons
    Sword,
    Axe,
    Hammer,
    Bow,
    Staff,
    Sceptre,
    // future weapons
    Dagger,
    Shield,
    Spear,
    Blowgun,
    // tools
    Debug,
    Farming,
    Pick,
    // npcs
    /// Intended for invisible weapons (e.g. a creature using its claws or
    /// biting)
    Natural,
    /// This is an placeholder item, it is used by non-humanoid npcs to attack
    Empty,
}

impl ToolKind {
    // Changing this will break persistence of modular weapons
    pub fn identifier_name(&self) -> &'static str {
        match self {
            ToolKind::Sword => "sword",
            ToolKind::Axe => "axe",
            ToolKind::Hammer => "hammer",
            ToolKind::Bow => "bow",
            ToolKind::Dagger => "dagger",
            ToolKind::Staff => "staff",
            ToolKind::Spear => "spear",
            ToolKind::Blowgun => "blowgun",
            ToolKind::Sceptre => "sceptre",
            ToolKind::Shield => "shield",
            ToolKind::Natural => "natural",
            ToolKind::Debug => "debug",
            ToolKind::Farming => "farming",
            ToolKind::Pick => "pickaxe",
            ToolKind::Empty => "empty",
        }
    }

    pub fn gains_combat_xp(&self) -> bool {
        matches!(
            self,
            ToolKind::Sword
                | ToolKind::Axe
                | ToolKind::Hammer
                | ToolKind::Bow
                | ToolKind::Dagger
                | ToolKind::Staff
                | ToolKind::Spear
                | ToolKind::Blowgun
                | ToolKind::Sceptre
                | ToolKind::Shield
        )
    }

    pub fn can_block(&self) -> bool {
        matches!(
            self,
            ToolKind::Sword
                | ToolKind::Axe
                | ToolKind::Hammer
                | ToolKind::Shield
                | ToolKind::Dagger
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hands {
    One,
    Two,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub equip_time_secs: f32,
    pub power: f32,
    pub effect_power: f32,
    pub speed: f32,
    pub crit_chance: f32,
    pub range: f32,
    pub energy_efficiency: f32,
    pub buff_strength: f32,
}

impl Stats {
    pub fn zero() -> Stats {
        Stats {
            equip_time_secs: 0.0,
            power: 0.0,
            effect_power: 0.0,
            speed: 0.0,
            crit_chance: 0.0,
            range: 0.0,
            energy_efficiency: 0.0,
            buff_strength: 0.0,
        }
    }

    pub fn one() -> Stats {
        Stats {
            equip_time_secs: 1.0,
            power: 1.0,
            effect_power: 1.0,
            speed: 1.0,
            crit_chance: 1.0,
            range: 1.0,
            energy_efficiency: 1.0,
            buff_strength: 1.0,
        }
    }
}

impl Asset for Stats {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl AddAssign<Stats> for Stats {
    fn add_assign(&mut self, other: Stats) {
        self.equip_time_secs += other.equip_time_secs;
        self.power += other.power;
        self.effect_power += other.effect_power;
        self.speed += other.speed;
        self.crit_chance += other.crit_chance;
        self.range += other.range;
        self.energy_efficiency += other.energy_efficiency;
        self.buff_strength += other.buff_strength;
    }
}
impl MulAssign<Stats> for Stats {
    fn mul_assign(&mut self, other: Stats) {
        self.equip_time_secs *= other.equip_time_secs;
        self.power *= other.power;
        self.effect_power *= other.effect_power;
        self.speed *= other.speed;
        self.crit_chance *= other.crit_chance;
        self.range *= other.range;
        self.energy_efficiency *= other.energy_efficiency;
        self.buff_strength *= other.buff_strength;
    }
}
impl Mul<Stats> for Stats {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self {
            equip_time_secs: self.equip_time_secs * other.equip_time_secs,
            power: self.power * other.power,
            effect_power: self.effect_power * other.effect_power,
            speed: self.speed * other.speed,
            crit_chance: self.crit_chance * other.crit_chance,
            range: self.range * other.range,
            energy_efficiency: self.energy_efficiency * other.energy_efficiency,
            buff_strength: self.buff_strength * other.buff_strength,
        }
    }
}
impl DivAssign<usize> for Stats {
    fn div_assign(&mut self, scalar: usize) {
        self.equip_time_secs /= scalar as f32;
        self.power /= scalar as f32;
        self.effect_power /= scalar as f32;
        self.speed /= scalar as f32;
        self.crit_chance /= scalar as f32;
        self.range /= scalar as f32;
        self.energy_efficiency /= scalar as f32;
        self.buff_strength /= scalar as f32;
    }
}

impl Sub<Stats> for Stats {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            equip_time_secs: self.equip_time_secs - other.equip_time_secs,
            power: self.power - other.power,
            effect_power: self.effect_power - other.effect_power,
            speed: self.speed - other.speed,
            crit_chance: self.crit_chance - other.crit_chance,
            range: self.range - other.range,
            energy_efficiency: self.range - other.energy_efficiency,
            buff_strength: self.buff_strength - other.buff_strength,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialStatManifest(pub HashMap<String, Stats>);

// This could be a Compound that also loads the keys, but the RecipeBook
// Compound impl already does that, so checking for existence here is redundant.
impl Asset for MaterialStatManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl Default for MaterialStatManifest {
    fn default() -> MaterialStatManifest {
        // TODO: Don't do this, loading a default should have no ability to panic
        MaterialStatManifest::load_expect_cloned("common.material_stats_manifest")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    pub kind: ToolKind,
    pub hands: Hands,
    pub stats: Stats,
    // TODO: item specific abilities
}

impl Tool {
    // DO NOT USE UNLESS YOU KNOW WHAT YOU ARE DOING
    // Added for CSV import of stats
    pub fn new(kind: ToolKind, hands: Hands, stats: Stats) -> Self { Self { kind, hands, stats } }

    pub fn empty() -> Self {
        Self {
            kind: ToolKind::Empty,
            hands: Hands::One,
            stats: Stats {
                equip_time_secs: 0.0,
                power: 1.00,
                effect_power: 1.00,
                speed: 1.00,
                crit_chance: 0.1,
                range: 1.0,
                energy_efficiency: 1.0,
                buff_strength: 1.0,
            },
        }
    }

    // Keep power between 0.5 and 2.00
    pub fn base_power(&self) -> f32 { self.stats.power }

    pub fn base_effect_power(&self) -> f32 { self.stats.effect_power }

    pub fn base_speed(&self) -> f32 {
        // Has floor to prevent infinite durations being created later down due to a
        // divide by zero
        self.stats.speed.max(0.1)
    }

    pub fn base_crit_chance(&self) -> f32 { self.stats.crit_chance }

    pub fn base_range(&self) -> f32 { self.stats.range }

    pub fn base_energy_efficiency(&self) -> f32 { self.stats.energy_efficiency }

    pub fn base_buff_strength(&self) -> f32 { self.stats.buff_strength }

    pub fn equip_time(&self) -> Duration { Duration::from_secs_f32(self.stats.equip_time_secs) }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilitySet<T> {
    pub primary: T,
    pub secondary: T,
    pub abilities: Vec<(Option<Skill>, T)>,
}

impl AbilitySet<AbilityItem> {
    #[must_use]
    pub fn modified_by_tool(self, tool: &Tool) -> Self {
        self.map(|a| AbilityItem {
            id: a.id,
            ability: a.ability.adjusted_by_stats(tool.stats),
        })
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

impl Default for AbilitySet<AbilityItem> {
    fn default() -> Self {
        AbilitySet {
            primary: AbilityItem {
                id: String::new(),
                ability: CharacterAbility::default(),
            },
            secondary: AbilityItem {
                id: String::new(),
                ability: CharacterAbility::default(),
            },
            abilities: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AbilitySpec {
    Tool(ToolKind),
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilityItem {
    pub id: String,
    pub ability: CharacterAbility,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilityMap<T = AbilityItem>(HashMap<AbilitySpec, AbilitySet<T>>);

impl Default for AbilityMap {
    fn default() -> Self {
        // TODO: Revert to old default
        if let Ok(map) = Self::load_cloned("common.abilities.ability_set_manifest") {
            map
        } else {
            let mut map = HashMap::new();
            map.insert(AbilitySpec::Tool(ToolKind::Empty), AbilitySet::default());
            AbilityMap(map)
        }
    }
}

impl<T> AbilityMap<T> {
    pub fn get_ability_set(&self, key: &AbilitySpec) -> Option<&AbilitySet<T>> { self.0.get(key) }
}

impl Asset for AbilityMap<String> {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl assets::Compound for AbilityMap {
    fn load<S: assets::source::Source + ?Sized>(
        cache: &assets::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, assets::BoxedError> {
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
                        set.map_ref(|s| AbilityItem {
                            id: s.clone(),
                            ability: cache.load_expect(s).cloned(),
                        }),
                    )
                })
                .collect(),
        ))
    }
}
