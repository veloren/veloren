// Note: If you changes here "break" old character saves you can change the
// version in voxygen\src\meta.rs in order to reset save files to being empty

use crate::{
    assets::{self, Asset, AssetExt, AssetHandle},
    comp::{
        ability::Stance, item::DurabilityMultiplier, skills::Skill, CharacterAbility, SkillSet,
    },
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Sub};

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
    /// Music Instruments
    Instrument,
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
            ToolKind::Spear => "spear",
            ToolKind::Blowgun => "blowgun",
            ToolKind::Sceptre => "sceptre",
            ToolKind::Shield => "shield",
            ToolKind::Natural => "natural",
            ToolKind::Debug => "debug",
            ToolKind::Farming => "farming",
            ToolKind::Pick => "pickaxe",
            ToolKind::Instrument => "instrument",
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

    /// Calculates a diminished buff strength where the buff strength is clamped
    /// by the power, and then excess buff strength above the power is added
    /// with diminishing returns.
    // TODO: Remove this later when there are more varied high tier materials.
    // Mainly exists for now as a hack to allow some progression in strength of
    // directly applied buffs.
    pub fn diminished_buff_strength(&self) -> f32 {
        let base = self.buff_strength.clamp(0.0, self.power);
        let diminished = (self.buff_strength - base + 1.0).log(5.0);
        base + diminished
    }

    pub fn with_durability_mult(&self, dur_mult: DurabilityMultiplier) -> Self {
        let less_scaled = dur_mult.0 * 0.5 + 0.5;
        Self {
            equip_time_secs: self.equip_time_secs / less_scaled.max(0.01),
            power: self.power * dur_mult.0,
            effect_power: self.effect_power * dur_mult.0,
            speed: self.speed * less_scaled,
            crit_chance: self.crit_chance * dur_mult.0,
            range: self.range * dur_mult.0,
            energy_efficiency: self.energy_efficiency * dur_mult.0,
            buff_strength: self.buff_strength * dur_mult.0,
        }
    }
}

impl Asset for Stats {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl Add<Stats> for Stats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            equip_time_secs: self.equip_time_secs + other.equip_time_secs,
            power: self.power + other.power,
            effect_power: self.effect_power + other.effect_power,
            speed: self.speed + other.speed,
            crit_chance: self.crit_chance + other.crit_chance,
            range: self.range + other.range,
            energy_efficiency: self.energy_efficiency + other.energy_efficiency,
            buff_strength: self.buff_strength + other.buff_strength,
        }
    }
}

impl AddAssign<Stats> for Stats {
    fn add_assign(&mut self, other: Stats) { *self = *self + other; }
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
            energy_efficiency: self.energy_efficiency - other.energy_efficiency,
            buff_strength: self.buff_strength - other.buff_strength,
        }
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

impl MulAssign<Stats> for Stats {
    fn mul_assign(&mut self, other: Stats) { *self = *self * other; }
}

impl Div<f32> for Stats {
    type Output = Self;

    fn div(self, scalar: f32) -> Self {
        Self {
            equip_time_secs: self.equip_time_secs / scalar,
            power: self.power / scalar,
            effect_power: self.effect_power / scalar,
            speed: self.speed / scalar,
            crit_chance: self.crit_chance / scalar,
            range: self.range / scalar,
            energy_efficiency: self.energy_efficiency / scalar,
            buff_strength: self.buff_strength / scalar,
        }
    }
}

impl Mul<DurabilityMultiplier> for Stats {
    type Output = Self;

    fn mul(self, value: DurabilityMultiplier) -> Self { self.with_durability_mult(value) }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    pub kind: ToolKind,
    pub hands: Hands,
    stats: Stats,
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

    pub fn stats(&self, durability_multiplier: DurabilityMultiplier) -> Stats {
        self.stats * durability_multiplier
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilitySet<T> {
    pub primary: AbilityKind<T>,
    pub secondary: AbilityKind<T>,
    pub abilities: Vec<AbilityKind<T>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AbilityKind<T> {
    Simple(Option<Skill>, T),
    Contextualized {
        pseudo_id: String,
        abilities: Vec<(Vec<AbilityContext>, (Option<Skill>, T))>,
    },
}

impl<T> AbilityKind<T> {
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> AbilityKind<U> {
        match self {
            Self::Simple(s, x) => AbilityKind::<U>::Simple(s, f(x)),
            Self::Contextualized {
                pseudo_id,
                abilities,
            } => AbilityKind::<U>::Contextualized {
                pseudo_id,
                abilities: abilities
                    .into_iter()
                    .map(|(c, (s, x))| (c, (s, f(x))))
                    .collect(),
            },
        }
    }

    pub fn map_ref<U, F: FnMut(&T) -> U>(&self, mut f: F) -> AbilityKind<U> {
        match self {
            Self::Simple(s, x) => AbilityKind::<U>::Simple(*s, f(x)),
            Self::Contextualized {
                pseudo_id,
                abilities,
            } => AbilityKind::<U>::Contextualized {
                pseudo_id: pseudo_id.clone(),
                abilities: abilities
                    .into_iter()
                    .map(|(c, (s, x))| (c.clone(), (*s, f(x))))
                    .collect(),
            },
        }
    }

    pub fn ability(&self, skillset: Option<&SkillSet>, contexts: &[AbilityContext]) -> Option<&T> {
        let unlocked = |s: Option<Skill>, a| {
            // If there is a skill requirement and the skillset does not contain the
            // required skill, return None
            s.map_or(true, |s| skillset.map_or(false, |ss| ss.has_skill(s)))
                .then_some(a)
        };

        match self {
            AbilityKind::Simple(s, a) => unlocked(*s, a),
            AbilityKind::Contextualized {
                pseudo_id: _,
                abilities,
            } => {
                // In the event that the ability from the current context is not unlocked with
                // the required skill, try falling back to the ability from this input that does
                // not require a context
                abilities
                    .iter()
                    .find_map(|(req_contexts, (s, a))| {
                        req_contexts
                            .iter()
                            .all(|req| contexts.contains(req))
                            .then_some((s, a))
                    })
                    .and_then(|(s, a)| unlocked(*s, a))
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, Eq, PartialEq, Hash)]
pub enum AbilityContext {
    /// Note, in this context `Stance::None` isn't intended to be used. e.g.
    /// `AbilityContext::None` should always be used instead of
    /// `AbilityContext::Stance(Stance::None)` in the ability map config
    /// files(s).
    Stance(Stance),
}

impl AbilityContext {
    pub fn from(stance: Option<&Stance>) -> Vec<Self> {
        let mut contexts = Vec::new();
        match stance {
            Some(Stance::None) => {},
            Some(stance) => contexts.push(AbilityContext::Stance(*stance)),
            None => {},
        }
        contexts
    }
}

impl AbilitySet<AbilityItem> {
    #[must_use]
    pub fn modified_by_tool(
        self,
        tool: &Tool,
        durability_multiplier: DurabilityMultiplier,
    ) -> Self {
        self.map(|a| AbilityItem {
            id: a.id,
            ability: a
                .ability
                .adjusted_by_stats(tool.stats(durability_multiplier)),
        })
    }
}

impl<T> AbilitySet<T> {
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> AbilitySet<U> {
        AbilitySet {
            primary: self.primary.map(&mut f),
            secondary: self.secondary.map(&mut f),
            abilities: self.abilities.into_iter().map(|x| x.map(&mut f)).collect(),
        }
    }

    pub fn map_ref<U, F: FnMut(&T) -> U>(&self, mut f: F) -> AbilitySet<U> {
        AbilitySet {
            primary: self.primary.map_ref(&mut f),
            secondary: self.secondary.map_ref(&mut f),
            abilities: self.abilities.iter().map(|x| x.map_ref(&mut f)).collect(),
        }
    }

    pub fn primary(&self, skillset: Option<&SkillSet>, contexts: &[AbilityContext]) -> Option<&T> {
        self.primary.ability(skillset, contexts)
    }

    pub fn secondary(
        &self,
        skillset: Option<&SkillSet>,
        contexts: &[AbilityContext],
    ) -> Option<&T> {
        self.secondary.ability(skillset, contexts)
    }

    pub fn auxiliary(
        &self,
        index: usize,
        skillset: Option<&SkillSet>,
        contexts: &[AbilityContext],
    ) -> Option<&T> {
        self.abilities
            .get(index)
            .and_then(|a| a.ability(skillset, contexts))
    }
}

impl Default for AbilitySet<AbilityItem> {
    fn default() -> Self {
        AbilitySet {
            primary: AbilityKind::Simple(None, AbilityItem {
                id: String::new(),
                ability: CharacterAbility::default(),
            }),
            secondary: AbilityKind::Simple(None, AbilityItem {
                id: String::new(),
                ability: CharacterAbility::default(),
            }),
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

impl AbilityMap {
    pub fn load() -> AssetHandle<Self> {
        Self::load_expect("common.abilities.ability_set_manifest")
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
    fn load(
        cache: assets::AnyCache,
        specifier: &assets::SharedString,
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
