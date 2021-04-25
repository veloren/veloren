// Note: If you changes here "break" old character saves you can change the
// version in voxygen\src\meta.rs in order to reset save files to being empty

use crate::{
    assets::{self, Asset, AssetExt},
    comp::{item::ItemKind, skills::Skill, CharacterAbility, Item},
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{
    ops::{AddAssign, DivAssign, MulAssign, Sub},
    time::Duration,
};
use tracing::error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    Sword,
    Axe,
    Hammer,
    HammerSimple, //simple tools utilized by small/large biped variants, to simplify movesets
    SwordSimple,
    StaffSimple,
    BowSimple,
    AxeSimple,
    Bow,
    Dagger,
    Staff,
    Sceptre,
    Shield,
    Spear,
    Unique(UniqueKind),
    Debug,
    Farming,
    Pick,
    /// This is an placeholder item, it is used by non-humanoid npcs to attack
    Empty,
}

impl ToolKind {
    pub fn identifier_name(&self) -> &'static str {
        match self {
            ToolKind::Sword => "sword",
            ToolKind::SwordSimple => "simple sword",
            ToolKind::Axe => "axe",
            ToolKind::AxeSimple => "simple axe",
            ToolKind::Hammer => "hammer",
            ToolKind::HammerSimple => "simple hammer",
            ToolKind::Bow => "bow",
            ToolKind::BowSimple => "simple bow",
            ToolKind::Dagger => "dagger",
            ToolKind::Staff => "staff",
            ToolKind::StaffSimple => "simple staff",
            ToolKind::Spear => "spear",
            ToolKind::Sceptre => "sceptre",
            ToolKind::Shield => "shield",
            ToolKind::Unique(_) => "unique",
            ToolKind::Debug => "debug",
            ToolKind::Farming => "farming",
            ToolKind::Pick => "pickaxe",
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
    pub equip_time_secs: f32,
    pub power: f32,
    pub poise_strength: f32,
    pub speed: f32,
    pub crit_chance: f32,
    pub crit_mult: f32,
}

impl Stats {
    pub fn zeroed() -> Stats {
        Stats {
            equip_time_secs: 0.0,
            power: 0.0,
            poise_strength: 0.0,
            speed: 0.0,
            crit_chance: 0.0,
            crit_mult: 0.0,
        }
    }

    pub fn clamp_speed(mut self) -> Stats {
        // if a tool has 0.0 speed, that panics due to being infinite duration, so
        // enforce speed >= 0.1 on the final product (but not the intermediates)
        self.speed = self.speed.max(0.1);
        self
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
        self.poise_strength += other.poise_strength;
        self.speed += other.speed;
        self.crit_chance += other.crit_chance;
        self.crit_mult += other.crit_mult;
    }
}
impl MulAssign<Stats> for Stats {
    fn mul_assign(&mut self, other: Stats) {
        self.equip_time_secs *= other.equip_time_secs;
        self.power *= other.power;
        self.poise_strength *= other.poise_strength;
        self.speed *= other.speed;
        self.crit_chance *= other.crit_chance;
        self.crit_mult *= other.crit_mult;
    }
}
impl DivAssign<usize> for Stats {
    fn div_assign(&mut self, scalar: usize) {
        self.equip_time_secs /= scalar as f32;
        // since averaging occurs when the stats are used multiplicatively, don't permit
        // multiplying an equip_time_secs by 0, since that would be overpowered
        self.equip_time_secs = self.equip_time_secs.max(0.001);
        self.power /= scalar as f32;
        self.poise_strength /= scalar as f32;
        self.speed /= scalar as f32;
        self.crit_chance /= scalar as f32;
        self.crit_mult /= scalar as f32;
    }
}

impl Sub<Stats> for Stats {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            equip_time_secs: self.equip_time_secs - other.equip_time_secs,
            power: self.power - other.power,
            poise_strength: self.poise_strength - other.poise_strength,
            speed: self.speed - other.speed,
            crit_chance: self.crit_chance - other.crit_chance,
            crit_mult: self.crit_mult - other.crit_mult,
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
        MaterialStatManifest::load_expect_cloned("common.material_stats_manifest")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum StatKind {
    Direct(Stats),
    Modular,
}

impl StatKind {
    pub fn resolve_stats(&self, msm: &MaterialStatManifest, components: &[Item]) -> Stats {
        let mut stats = match self {
            StatKind::Direct(stats) => *stats,
            StatKind::Modular => Stats::zeroed(),
        };
        let mut multipliers: Vec<Stats> = Vec::new();
        for item in components.iter() {
            match item.kind() {
                ItemKind::ModularComponent(mc) => {
                    let inner_stats =
                        StatKind::Direct(mc.stats).resolve_stats(msm, item.components());
                    stats += inner_stats;
                },
                ItemKind::Ingredient { .. } => {
                    if let Some(mult_stats) = msm.0.get(item.item_definition_id()) {
                        multipliers.push(*mult_stats);
                    }
                },
                // TODO: add stats from enhancement slots
                _ => (),
            }
        }
        // Take the average of the material multipliers, to allow alloyed blades
        if !multipliers.is_empty() {
            let mut average_mult = Stats::zeroed();
            for stat in multipliers.iter() {
                average_mult += *stat;
            }
            average_mult /= multipliers.len();
            stats *= average_mult;
        }
        stats
    }
}

impl From<(&MaterialStatManifest, &[Item], &Tool)> for Stats {
    fn from((msm, components, tool): (&MaterialStatManifest, &[Item], &Tool)) -> Self {
        let raw_stats = tool.stats.resolve_stats(msm, components).clamp_speed();
        let (power, speed, poise) = match tool.hands {
            Hands::One => (0.67, 1.33, 0.67),
            Hands::Two => (1.5, 0.75, 1.5),
        };
        Self {
            equip_time_secs: raw_stats.equip_time_secs,
            power: raw_stats.power * power,
            poise_strength: raw_stats.poise_strength * poise,
            speed: raw_stats.speed * speed,
            crit_chance: raw_stats.crit_chance,
            crit_mult: raw_stats.crit_mult,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: ToolKind,
        hands: Hands,
        equip_time_secs: f32,
        power: f32,
        poise_strength: f32,
        speed: f32,
        crit_chance: f32,
        crit_mult: f32,
    ) -> Self {
        Self {
            kind,
            hands,
            stats: StatKind::Direct(Stats {
                equip_time_secs,
                power,
                poise_strength,
                speed,
                crit_chance,
                crit_mult,
            }),
        }
    }

    pub fn empty() -> Self {
        Self {
            kind: ToolKind::Empty,
            hands: Hands::One,
            stats: StatKind::Direct(Stats {
                equip_time_secs: 0.0,
                power: 1.00,
                poise_strength: 1.00,
                speed: 1.00,
                crit_chance: 0.1,
                crit_mult: 2.0,
            }),
        }
    }

    // Keep power between 0.5 and 2.00
    pub fn base_power(&self, msm: &MaterialStatManifest, components: &[Item]) -> f32 {
        self.stats.resolve_stats(msm, components).power
    }

    pub fn base_poise_strength(&self, msm: &MaterialStatManifest, components: &[Item]) -> f32 {
        self.stats.resolve_stats(msm, components).poise_strength
    }

    pub fn base_speed(&self, msm: &MaterialStatManifest, components: &[Item]) -> f32 {
        self.stats
            .resolve_stats(msm, components)
            .clamp_speed()
            .speed
    }

    pub fn base_crit_chance(&self, msm: &MaterialStatManifest, components: &[Item]) -> f32 {
        self.stats.resolve_stats(msm, components).crit_chance
    }

    pub fn base_crit_mult(&self, msm: &MaterialStatManifest, components: &[Item]) -> f32 {
        self.stats.resolve_stats(msm, components).crit_mult
    }

    pub fn equip_time(&self, msm: &MaterialStatManifest, components: &[Item]) -> Duration {
        Duration::from_secs_f32(self.stats.resolve_stats(msm, components).equip_time_secs)
    }

    pub fn get_abilities(
        &self,
        msm: &MaterialStatManifest,
        components: &[Item],
        map: &AbilityMap,
    ) -> AbilitySet<CharacterAbility> {
        if let Some(set) = map.0.get(&self.kind).cloned() {
            set.modified_by_tool(&self, msm, components)
        } else {
            error!(
                "ToolKind: {:?} has no AbilitySet in the ability map falling back to default",
                &self.kind
            );
            Default::default()
        }
    }

    pub fn can_block(&self) -> bool {
        matches!(
            self.kind,
            ToolKind::Sword
                | ToolKind::Axe
                | ToolKind::Hammer
                | ToolKind::Shield
                | ToolKind::Dagger
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilitySet<T> {
    pub primary: T,
    pub secondary: T,
    pub abilities: Vec<(Option<Skill>, T)>,
}

impl AbilitySet<CharacterAbility> {
    pub fn modified_by_tool(
        self,
        tool: &Tool,
        msm: &MaterialStatManifest,
        components: &[Item],
    ) -> Self {
        let stats = Stats::from((msm, components, tool));
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
    Husk,
    BeastClaws,
    WendigoMagic,
    TidalClaws,
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
    QuadLowBeam,
    QuadSmallBasic,
    TheropodBasic,
    TheropodBird,
    TheropodCharge,
    ObjectTurret,
    WoodenSpear,
    MindflayerStaff,
    BirdLargeBreathe,
    BirdLargeFire,
}
