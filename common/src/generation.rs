use std::ops::RangeInclusive;

use crate::{
    assets::{self, AssetExt, Error},
    calendar::Calendar,
    combat::{DeathEffect, DeathEffects, RiderEffects},
    comp::{
        Alignment, Body, Item, agent, humanoid,
        inventory::loadout_builder::{LoadoutBuilder, LoadoutSpec},
        misc::PortalData,
    },
    effect::BuffEffect,
    lottery::LootSpec,
    npc::{self, NPC_NAMES},
    resources::TimeOfDay,
    rtsim,
    trade::SiteInformation,
};
use common_base::dev_panic;
use common_i18n::Content;
use enum_map::EnumMap;
use serde::Deserialize;
use tracing::error;
use vek::*;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub enum NameKind {
    /// i18n key with attributes for feminine and masculine versions
    Translate(String),
    /// Derive the name from the `Body` using [`NPC_NAMES`] manifest.
    /// Also see `npc_names.ron`.
    Automatic,
    /// Explicitly state no name
    Uninit,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub enum BodyBuilder {
    RandomWith(String),
    Exact(Body),
    Uninit,
}

#[derive(Debug, Deserialize, Clone)]
pub enum AlignmentMark {
    Alignment(Alignment),
    Uninit,
}

impl Default for AlignmentMark {
    fn default() -> Self { Self::Alignment(Alignment::Wild) }
}

#[derive(Default, Debug, Deserialize, Clone)]
#[serde(default)]
pub struct AgentConfig {
    pub has_agency: Option<bool>,
    pub no_flee: Option<bool>,
    pub idle_wander_factor: Option<f32>,
    pub aggro_range_multiplier: Option<f32>,
}

#[derive(Debug, Deserialize, Clone)]
pub enum LoadoutKind {
    FromBody,
    Asset(String),
    Inline(Box<LoadoutSpec>),
}

#[derive(Debug, Deserialize, Clone)]
pub struct InventorySpec {
    pub loadout: LoadoutKind,
    #[serde(default)]
    pub items: Vec<(u32, String)>,
}

#[derive(Debug, Deserialize, Clone)]
pub enum Meta {
    SkillSetAsset(String),
}

// FIXME: currently this is used for both base definition
// and extension manifest.
// This is why all fields have Uninit kind which is means
// that this field should be either Default or Unchanged
// depending on how it is used.
//
// When we will use extension manifests more, it would be nicer to
// split EntityBase and EntityExtension to different structs.
//
// Fields which have Uninit enum kind
// should be optional (or renamed to Unchanged) in EntityExtension
// and required (or renamed to Default) in EntityBase
/// Struct for EntityInfo manifest.
///
/// Intended to use with .ron files as base definition or
/// in rare cases as extension manifest.
/// Pure data struct, doesn't do anything until evaluated with EntityInfo.
///
/// Check assets/common/entity/template.ron or other examples.
///
/// # Example
/// ```
/// use vek::Vec3;
/// use veloren_common::generation::EntityInfo;
///
/// // create new EntityInfo at dummy position
/// // and fill it with template config
/// let dummy_position = Vec3::new(0.0, 0.0, 0.0);
/// // rng is required because some elements may be randomly generated
/// let mut dummy_rng = rand::rng();
/// let entity = EntityInfo::at(dummy_position).with_asset_expect(
///     "common.entity.template",
///     &mut dummy_rng,
///     None,
/// );
/// ```
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct EntityConfig {
    /// Name of Entity
    /// Can be Name(String) with given name
    /// or Automatic which will call automatic name depend on Body
    /// or Uninit (means it should be specified somewhere in code)
    // Hidden, because its behaviour depends on `body` field.
    name: NameKind,

    /// Body
    /// Can be Exact (Body with all fields e.g BodyType, Species, Hair color and
    /// such) or RandomWith (will generate random body or species)
    /// or Uninit (means it should be specified somewhere in code)
    pub body: BodyBuilder,

    /// Alignment, can be Uninit
    pub alignment: AlignmentMark,

    /// Parameterises agent behaviour
    #[serde(default)]
    pub agent: AgentConfig,

    /// Loot
    /// See LootSpec in lottery
    pub loot: LootSpec<String>,

    /// Loadout & Inventory
    /// Check docs for `InventorySpec` struct in this file.
    pub inventory: InventorySpec,

    /// Pets to spawn with this entity (specified as a list of asset paths).
    /// The range represents how many pets will be spawned.
    #[serde(default)]
    pub pets: Vec<(String, RangeInclusive<usize>)>,

    /// If this entity spawns with a rider.
    #[serde(default)]
    pub rider: Option<String>,

    /// Buffs this entity gives to whatever is riding it.
    #[serde(default)]
    pub rider_effects: Vec<BuffEffect>,

    #[serde(default = "num_traits::One::one")]
    pub scale: f32,

    #[serde(default)]
    pub death_effects: Vec<DeathEffect>,

    /// Meta Info for optional fields
    /// Possible fields:
    /// SkillSetAsset(String) with asset_specifier for skillset
    #[serde(default)]
    pub meta: Vec<Meta>,
}

impl assets::FileAsset for EntityConfig {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Result<Self, assets::BoxedError> {
        assets::load_ron(&bytes)
    }
}

impl EntityConfig {
    pub fn from_asset_expect_owned(asset_specifier: &str) -> Self {
        Self::load_owned(asset_specifier)
            .unwrap_or_else(|e| panic!("Failed to load {}. Error: {:?}", asset_specifier, e))
    }

    #[must_use]
    pub fn with_body(mut self, body: BodyBuilder) -> Self {
        self.body = body;

        self
    }
}

/// Return all entity config specifiers
pub fn try_all_entity_configs() -> Result<Vec<String>, Error> {
    let configs = assets::load_rec_dir::<EntityConfig>("common.entity")?;
    Ok(configs.read().ids().map(|id| id.to_string()).collect())
}

#[derive(Clone, Debug)]
pub enum SpecialEntity {
    Waypoint,
    Teleporter(PortalData),
    /// Totem with FriendlyFire and ForcePvP auras
    ArenaTotem {
        range: f32,
    },
}

#[derive(Clone)]
pub struct EntityInfo {
    pub pos: Vec3<f32>,
    pub alignment: Alignment,
    /// Parameterises agent behaviour
    pub has_agency: bool,
    pub agent_mark: Option<agent::Mark>,
    pub no_flee: bool,
    pub idle_wander_factor: f32,
    pub aggro_range_multiplier: f32,
    // Stats
    pub body: Body,
    pub name: Option<Content>,
    pub scale: f32,
    // Loot
    pub loot: LootSpec<String>,
    // Loadout
    pub inventory: Vec<(u32, Item)>,
    pub loadout: LoadoutBuilder,
    pub make_loadout: Option<
        fn(
            LoadoutBuilder,
            Option<&SiteInformation>,
            time: Option<&(TimeOfDay, Calendar)>,
        ) -> LoadoutBuilder,
    >,
    // Skills
    pub skillset_asset: Option<String>,
    pub death_effects: Option<DeathEffects>,
    pub rider_effects: Option<RiderEffects>,

    pub pets: Vec<EntityInfo>,

    pub rider: Option<Box<EntityInfo>>,

    // Economy
    // we can't use DHashMap, do we want to move that into common?
    pub trading_information: Option<SiteInformation>,
    //Option<hashbrown::HashMap<crate::trade::Good, (f32, f32)>>, /* price and available amount */

    // Edge cases, override everything else
    pub special_entity: Option<SpecialEntity>,
}

impl EntityInfo {
    pub fn at(pos: Vec3<f32>) -> Self {
        Self {
            pos,
            alignment: Alignment::Wild,

            has_agency: true,
            agent_mark: None,
            no_flee: false,
            idle_wander_factor: 1.0,
            aggro_range_multiplier: 1.0,

            body: Body::Humanoid(humanoid::Body::random()),
            name: None,
            scale: 1.0,
            loot: LootSpec::Nothing,
            inventory: Vec::new(),
            loadout: LoadoutBuilder::empty(),
            make_loadout: None,
            death_effects: None,
            rider_effects: None,
            skillset_asset: None,
            pets: Vec::new(),
            rider: None,
            trading_information: None,
            special_entity: None,
        }
    }

    /// Helper function for applying config from asset
    /// with specified Rng for managing loadout.
    #[must_use]
    pub fn with_asset_expect<R>(
        self,
        asset_specifier: &str,
        loadout_rng: &mut R,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Self
    where
        R: rand::Rng,
    {
        let config = EntityConfig::load_expect_cloned(asset_specifier);

        self.with_entity_config(config, Some(asset_specifier), loadout_rng, time)
    }

    /// Evaluate and apply EntityConfig
    #[must_use]
    pub fn with_entity_config<R>(
        mut self,
        config: EntityConfig,
        config_asset: Option<&str>,
        loadout_rng: &mut R,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Self
    where
        R: rand::Rng,
    {
        let EntityConfig {
            name,
            body,
            alignment,
            agent,
            inventory,
            loot,
            meta,
            scale,
            pets,
            rider,
            death_effects,
            rider_effects,
        } = config;

        match body {
            BodyBuilder::RandomWith(string) => {
                let npc::NpcBody(_body_kind, mut body_creator) =
                    string.parse::<npc::NpcBody>().unwrap_or_else(|err| {
                        panic!("failed to parse body {:?}. Err: {:?}", &string, err)
                    });
                let body = body_creator();
                self = self.with_body(body);
            },
            BodyBuilder::Exact(body) => {
                self = self.with_body(body);
            },
            BodyBuilder::Uninit => {},
        }

        // NOTE: set name after body, as body is needed used with for both
        // automatic and translated names
        match name {
            NameKind::Translate(key) => {
                let name = Content::with_attr(key, self.body.gender_attr());
                self = self.with_name(name);
            },
            NameKind::Automatic => {
                self = self.with_automatic_name();
            },
            NameKind::Uninit => {},
        }

        if let AlignmentMark::Alignment(alignment) = alignment {
            self = self.with_alignment(alignment);
        }

        self = self.with_loot_drop(loot);

        // NOTE: set loadout after body, as it's used with default equipement
        self = self.with_inventory(inventory, config_asset, loadout_rng, time);

        let mut pet_infos: Vec<EntityInfo> = Vec::new();
        for (pet_asset, amount) in pets {
            let config = EntityConfig::load_expect(&pet_asset).read();
            let (start, mut end) = amount.into_inner();
            if start > end {
                error!("Invalid range for pet count start: {start}, end: {end}");
                end = start;
            }

            pet_infos.extend((0..loadout_rng.random_range(start..=end)).map(|_| {
                EntityInfo::at(self.pos).with_entity_config(
                    config.clone(),
                    config_asset,
                    loadout_rng,
                    time,
                )
            }));
        }
        self.scale = scale;

        self.pets = pet_infos;

        self.rider = rider.map(|rider| {
            let config = EntityConfig::load_expect(&rider).read();
            Box::new(EntityInfo::at(self.pos).with_entity_config(
                config.clone(),
                config_asset,
                loadout_rng,
                time,
            ))
        });

        // Prefer the new configuration, if possible
        let AgentConfig {
            has_agency,
            no_flee,
            idle_wander_factor,
            aggro_range_multiplier,
        } = agent;
        self.has_agency = has_agency.unwrap_or(self.has_agency);
        self.no_flee = no_flee.unwrap_or(self.no_flee);
        self.idle_wander_factor = idle_wander_factor.unwrap_or(self.idle_wander_factor);
        self.aggro_range_multiplier = aggro_range_multiplier.unwrap_or(self.aggro_range_multiplier);
        self.death_effects = (!death_effects.is_empty()).then_some(DeathEffects(death_effects));
        self.rider_effects = (!rider_effects.is_empty()).then_some(RiderEffects(rider_effects));

        for field in meta {
            match field {
                Meta::SkillSetAsset(asset) => {
                    self = self.with_skillset_asset(asset);
                },
            }
        }

        self
    }

    /// Return EntityInfo with LoadoutBuilder and items overwritten
    // NOTE: helper function, think twice before exposing it
    #[must_use]
    fn with_inventory<R>(
        mut self,
        inventory: InventorySpec,
        config_asset: Option<&str>,
        rng: &mut R,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Self
    where
        R: rand::Rng,
    {
        let config_asset = config_asset.unwrap_or("???");
        let InventorySpec { loadout, items } = inventory;

        // FIXME: this shouldn't always overwrite
        // inventory. Think about this when we get to
        // entity config inheritance.
        self.inventory = items
            .into_iter()
            .map(|(num, i)| (num, Item::new_from_asset_expect(&i)))
            .collect();

        match loadout {
            LoadoutKind::FromBody => {
                self = self.with_default_equip();
            },
            LoadoutKind::Asset(loadout) => {
                let loadout = LoadoutBuilder::from_asset(&loadout, rng, time).unwrap_or_else(|e| {
                    panic!("failed to load loadout for {config_asset}: {e:?}");
                });
                self.loadout = loadout;
            },
            LoadoutKind::Inline(loadout_spec) => {
                let loadout = LoadoutBuilder::from_loadout_spec(*loadout_spec, rng, time)
                    .unwrap_or_else(|e| {
                        panic!("failed to load loadout for {config_asset}: {e:?}");
                    });
                self.loadout = loadout;
            },
        }

        self
    }

    /// Return EntityInfo with LoadoutBuilder overwritten
    // NOTE: helper function, think twice before exposing it
    #[must_use]
    fn with_default_equip(mut self) -> Self {
        let loadout_builder = LoadoutBuilder::from_default(&self.body);
        self.loadout = loadout_builder;

        self
    }

    #[must_use]
    pub fn do_if(mut self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond {
            self = f(self);
        }
        self
    }

    #[must_use]
    pub fn into_special(mut self, special: SpecialEntity) -> Self {
        self.special_entity = Some(special);
        self
    }

    #[must_use]
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    #[must_use]
    pub fn with_body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    #[must_use]
    pub fn with_name(mut self, name: Content) -> Self {
        self.name = Some(name);
        self
    }

    #[must_use]
    pub fn with_agency(mut self, agency: bool) -> Self {
        self.has_agency = agency;
        self
    }

    #[must_use]
    pub fn with_agent_mark(mut self, agent_mark: impl Into<Option<agent::Mark>>) -> Self {
        self.agent_mark = agent_mark.into();
        self
    }

    #[must_use]
    pub fn with_loot_drop(mut self, loot_drop: LootSpec<String>) -> Self {
        self.loot = loot_drop;
        self
    }

    #[must_use]
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    #[must_use]
    pub fn with_lazy_loadout(
        mut self,
        creator: fn(
            LoadoutBuilder,
            Option<&SiteInformation>,
            time: Option<&(TimeOfDay, Calendar)>,
        ) -> LoadoutBuilder,
    ) -> Self {
        self.make_loadout = Some(creator);
        self
    }

    #[must_use]
    pub fn with_skillset_asset(mut self, asset: String) -> Self {
        self.skillset_asset = Some(asset);
        self
    }

    #[must_use]
    pub fn with_automatic_name(mut self) -> Self {
        let npc_names = NPC_NAMES.read();
        self.name = npc_names.get_default_name(&self.body);
        self
    }

    #[must_use]
    pub fn with_alias(mut self, alias: String) -> Self {
        self.name = Some(Content::localized_with_args(
            "name-misc-with-alias-template",
            [
                ("alias", Content::Plain(alias)),
                (
                    "old_name",
                    self.name.unwrap_or_else(|| {
                        dev_panic!("no name present to use with with_alias");
                        Content::Plain("??".to_owned())
                    }),
                ),
            ],
        ));
        self
    }

    /// map contains price+amount
    #[must_use]
    pub fn with_economy<'a>(mut self, e: impl Into<Option<&'a SiteInformation>>) -> Self {
        self.trading_information = e.into().cloned();
        self
    }

    #[must_use]
    pub fn with_no_flee(mut self) -> Self {
        self.no_flee = true;
        self
    }

    #[must_use]
    pub fn with_loadout(mut self, loadout: LoadoutBuilder) -> Self {
        self.loadout = loadout;
        self
    }
}

#[derive(Default)]
pub struct ChunkSupplement {
    pub entities: Vec<EntityInfo>,
    pub rtsim_max_resources: EnumMap<rtsim::ChunkResource, usize>,
}

impl ChunkSupplement {
    pub fn add_entity(&mut self, entity: EntityInfo) { self.entities.push(entity); }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::SkillSetBuilder;
    use hashbrown::HashMap;

    #[derive(Debug, Eq, Hash, PartialEq)]
    enum MetaId {
        SkillSetAsset,
    }

    impl Meta {
        fn id(&self) -> MetaId {
            match self {
                Meta::SkillSetAsset(_) => MetaId::SkillSetAsset,
            }
        }
    }

    #[cfg(test)]
    fn validate_body(body: &BodyBuilder, config_asset: &str) {
        match body {
            BodyBuilder::RandomWith(string) => {
                let npc::NpcBody(_body_kind, mut body_creator) =
                    string.parse::<npc::NpcBody>().unwrap_or_else(|err| {
                        panic!(
                            "failed to parse body {:?} in {}. Err: {:?}",
                            &string, config_asset, err
                        )
                    });
                let _ = body_creator();
            },
            BodyBuilder::Uninit | BodyBuilder::Exact { .. } => {},
        }
    }

    #[cfg(test)]
    fn validate_inventory(inventory: InventorySpec, body: &BodyBuilder, config_asset: &str) {
        let InventorySpec { loadout, items } = inventory;

        match loadout {
            LoadoutKind::FromBody => {
                if body.clone() == BodyBuilder::Uninit {
                    // there is a big chance to call automatic name
                    // when body is yet undefined
                    panic!("Used FromBody loadout with Uninit body in {}", config_asset);
                }
            },
            LoadoutKind::Asset(asset) => {
                let loadout =
                    LoadoutSpec::load_cloned(&asset).expect("failed to load loadout asset");
                loadout
                    .validate(vec![asset])
                    .unwrap_or_else(|e| panic!("Config {config_asset} is broken: {e:?}"));
            },
            LoadoutKind::Inline(spec) => {
                spec.validate(Vec::new())
                    .unwrap_or_else(|e| panic!("Config {config_asset} is broken: {e:?}"));
            },
        }

        // TODO: check for number of items
        //
        // 1) just with 16 default slots?
        // - well, keep in mind that not every item can stack to infinite amount
        //
        // 2) discover total capacity from loadout?
        for (num, item_str) in items {
            let item = Item::new_from_asset(&item_str);
            let mut item = item.unwrap_or_else(|err| {
                panic!("can't load {} in {}: {:?}", item_str, config_asset, err);
            });
            item.set_amount(num).unwrap_or_else(|err| {
                panic!(
                    "can't set amount {} for {} in {}: {:?}",
                    num, item_str, config_asset, err
                );
            });
        }
    }

    #[cfg(test)]
    fn validate_name(name: NameKind, body: BodyBuilder, config_asset: &str) {
        if (name == NameKind::Automatic || matches!(name, NameKind::Translate(_)))
            && body == BodyBuilder::Uninit
        {
            // there is a big chance to call automatic name
            // when body is yet undefined
            //
            // use .with_automatic_name() in code explicitly
            panic!(
                "Used Automatic/Translate name with Uninit body in {}",
                config_asset
            );
        }
    }

    #[cfg(test)]
    fn validate_loot(loot: LootSpec<String>, _config_asset: &str) {
        use crate::lottery;
        lottery::tests::validate_loot_spec(&loot);
    }

    #[cfg(test)]
    fn validate_meta(meta: Vec<Meta>, config_asset: &str) {
        let mut meta_counter = HashMap::new();
        for field in meta {
            meta_counter
                .entry(field.id())
                .and_modify(|c| *c += 1)
                .or_insert(1);

            match field {
                Meta::SkillSetAsset(asset) => {
                    drop(SkillSetBuilder::from_asset_expect(&asset));
                },
            }
        }
        for (meta_id, counter) in meta_counter {
            if counter > 1 {
                panic!("Duplicate {:?} in {}", meta_id, config_asset);
            }
        }
    }

    #[cfg(test)]
    fn validate_pets(pets: Vec<(String, RangeInclusive<usize>)>, config_asset: &str) {
        for (pet, amount) in pets.into_iter().map(|(pet_asset, amount)| {
            (
                EntityConfig::load_cloned(&pet_asset).unwrap_or_else(|_| {
                    panic!("Pet asset path invalid: \"{pet_asset}\", in {config_asset}")
                }),
                amount,
            )
        }) {
            assert!(
                amount.end() >= amount.start(),
                "Invalid pet spawn range ({}..={}), in {}",
                amount.start(),
                amount.end(),
                config_asset
            );
            if !pet.pets.is_empty() {
                panic!("Pets must not be owners of pets: {config_asset}");
            }
        }
    }

    #[cfg(test)]
    fn validate_death_effects(effects: Vec<DeathEffect>, config_asset: &str) {
        for effect in effects {
            match effect {
                DeathEffect::AttackerBuff {
                    kind: _,
                    strength: _,
                    duration: _,
                } => {},
                DeathEffect::Transform {
                    entity_spec,
                    allow_players: _,
                } => {
                    if let Err(error) = EntityConfig::load(&entity_spec) {
                        panic!(
                            "Error while loading transform entity spec ({entity_spec}) for entity \
                             {config_asset}: {error:?}"
                        );
                    }
                },
            }
        }
    }

    fn validate_rider(rider: Option<String>, config_asset: &str) {
        if let Some(rider) = rider {
            EntityConfig::load_cloned(&rider).unwrap_or_else(|_| {
                panic!("Rider asset path invalid: \"{rider}\", in {config_asset}")
            });
        }
    }

    #[cfg(test)]
    pub fn validate_entity_config(config_asset: &str) {
        let EntityConfig {
            body,
            inventory,
            name,
            loot,
            pets,
            rider,
            meta,
            death_effects,
            alignment: _, // can't fail if serialized, it's a boring enum
            rider_effects: _,
            scale,
            agent: _,
        } = EntityConfig::from_asset_expect_owned(config_asset);

        assert!(
            scale.is_finite() && scale > 0.0,
            "Scale has to be finite and greater than zero"
        );

        validate_body(&body, config_asset);
        // body dependent stuff
        validate_inventory(inventory, &body, config_asset);
        validate_name(name, body, config_asset);
        // misc
        validate_loot(loot, config_asset);
        validate_meta(meta, config_asset);
        validate_pets(pets, config_asset);
        validate_rider(rider, config_asset);
        validate_death_effects(death_effects, config_asset);
    }

    #[test]
    fn test_all_entity_assets() {
        // Get list of entity configs, load everything, validate content.
        let entity_configs =
            try_all_entity_configs().expect("Failed to access entity configs directory");
        for config_asset in entity_configs {
            validate_entity_config(&config_asset)
        }
    }
}
