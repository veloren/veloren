use crate::{
    assets::{self, AssetExt, Error},
    comp::{
        self, agent, humanoid,
        inventory::loadout_builder::{ItemSpec, LoadoutBuilder},
        Alignment, Body, Item,
    },
    lottery::{LootSpec, Lottery},
    npc::{self, NPC_NAMES},
    trade,
    trade::SiteInformation,
};
use serde::Deserialize;
use vek::*;

#[derive(Debug, Deserialize, Clone)]
enum NameKind {
    Name(String),
    Automatic,
    Uninit,
}

#[derive(Debug, Deserialize, Clone)]
enum BodyBuilder {
    RandomWith(String),
    Exact(Body),
    Uninit,
}

#[derive(Debug, Deserialize, Clone)]
enum AlignmentMark {
    Alignment(Alignment),
    Uninit,
}

#[derive(Debug, Deserialize, Clone)]
enum LootKind {
    Item(String),
    LootTable(String),
    Uninit,
}

#[derive(Debug, Deserialize, Clone)]
enum Hands {
    TwoHanded(ItemSpec),
    Paired(ItemSpec),
    Mix {
        mainhand: ItemSpec,
        offhand: ItemSpec,
    },
    Uninit,
}

#[derive(Debug, Deserialize, Clone)]
enum Meta {
    LoadoutAsset(String),
    SkillSetAsset(String),
}

// FIXME: currently this is used for both base definition
// and extension manifest.
// This is why all fields have Uninit kind which is means
// that this field should be either Default or Unchanged
// depending on how it is used.
//
// When we will use exension manifests more, it would be nicer to
// split EntityBase and EntityExtension to different structs.
//
// Fields which have Uninit enum kind
// should be optional (or renamed to Unchanged) in EntityExtension
// and required (or renamed to Default) in EntityBase
/// Struct for EntityInfo manifest.
///
/// Intended to use with .ron files as base definion or
/// in rare cases as extension manifest.
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
/// let entity = EntityInfo::at(dummy_position).with_asset_expect("common.entity.template");
/// ```
#[derive(Debug, Deserialize, Clone)]
pub struct EntityConfig {
    /// Name of Entity
    /// Can be Name(String) with given name
    /// or Automatic which will call automatic name depend on Body
    /// or Uninit (means it should be specified somewhere in code)
    name: NameKind,

    /// Body
    /// Can be Exact (Body with all fields e.g BodyType, Species, Hair color and
    /// such) or RandomWith (will generate random body or species)
    /// or Uninit (means it should be specified somewhere in code)
    body: BodyBuilder,

    /// Alignment, can be Uninit
    alignment: AlignmentMark,

    /// Loot
    /// Can be Item (with asset_specifier for item)
    /// or LootTable (with asset_specifier for loot table)
    /// or Uninit (means it should be specified something in the code)
    loot: LootKind,

    /// Hands:
    /// - TwoHanded(ItemSpec) for one 2h or 1h weapon,
    /// - Paired(ItemSpec) for two 1h weapons aka berserker mode,
    /// - Mix { mainhand: ItemSpec, offhand: ItemSpec,
    /// } for two different 1h weapons,
    /// - Uninit which means that tool should be specified somewhere in code,
    /// Where ItemSpec is taken from loadout_builder module
    // TODO: better name for this?
    // wielding? equipped? what do you think Tigers are wielding?
    // should we use this field for animals without visible weapons at all?
    hands: Hands,

    /// Meta Info for optional fields
    /// Possible fields:
    /// LoadoutAsset(String) with asset_specifier for loadout
    /// SkillSetAsset(String) with asset_specifier for skillset
    meta: Vec<Meta>,
}

impl assets::Asset for EntityConfig {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl EntityConfig {
    pub fn from_asset_expect(asset_specifier: &str) -> EntityConfig {
        Self::load_owned(asset_specifier)
            .unwrap_or_else(|e| panic!("Failed to load {}. Error: {}", asset_specifier, e))
    }
}

/// Return all entity config specifiers
pub fn try_all_entity_configs() -> Result<Vec<String>, Error> {
    let configs = assets::load_dir::<EntityConfig>("common.entity", true)?;
    Ok(configs.ids().map(|id| id.to_owned()).collect())
}

#[derive(Clone)]
pub struct EntityInfo {
    pub pos: Vec3<f32>,
    pub is_waypoint: bool, // Edge case, overrides everything else
    pub is_giant: bool,
    pub has_agency: bool,
    pub alignment: Alignment,
    pub agent_mark: Option<agent::Mark>,
    pub body: Body,
    pub name: Option<String>,
    pub main_tool: Option<Item>,
    pub second_tool: Option<Item>,
    pub scale: f32,
    // TODO: Properly give NPCs skills
    pub level: Option<u16>,
    pub loot_drop: Option<Item>,
    pub loadout_asset: Option<String>,
    pub make_loadout: Option<fn(LoadoutBuilder, Option<&trade::SiteInformation>) -> LoadoutBuilder>,
    pub skillset_asset: Option<String>,
    pub pet: Option<Box<EntityInfo>>,
    // we can't use DHashMap, do we want to move that into common?
    pub trading_information: Option<trade::SiteInformation>,
    //Option<hashbrown::HashMap<crate::trade::Good, (f32, f32)>>, /* price and available amount */
}

impl EntityInfo {
    pub fn at(pos: Vec3<f32>) -> Self {
        Self {
            pos,
            is_waypoint: false,
            is_giant: false,
            has_agency: true,
            alignment: Alignment::Wild,
            agent_mark: None,
            body: Body::Humanoid(humanoid::Body::random()),
            name: None,
            main_tool: None,
            second_tool: None,
            scale: 1.0,
            level: None,
            loot_drop: None,
            loadout_asset: None,
            make_loadout: None,
            skillset_asset: None,
            pet: None,
            trading_information: None,
        }
    }

    pub fn with_asset_expect(self, asset_specifier: &str) -> Self {
        let config = EntityConfig::load_expect(asset_specifier).read().clone();

        self.with_entity_config(config, Some(asset_specifier))
    }

    // helper function to apply config
    fn with_entity_config(mut self, config: EntityConfig, config_asset: Option<&str>) -> Self {
        let EntityConfig {
            name,
            body,
            alignment,
            loot,
            hands,
            meta,
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

        // NOTE: set name after body, as it's used with automatic name
        match name {
            NameKind::Name(name) => {
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

        match loot {
            LootKind::Item(asset) => {
                self = self.with_loot_drop(Item::new_from_asset_expect(&asset));
            },
            LootKind::LootTable(asset) => {
                let table = Lottery::<LootSpec<String>>::load_expect(&asset);
                let drop = table.read().choose().to_item();
                self = self.with_loot_drop(drop);
            },
            LootKind::Uninit => {},
        }

        let rng = &mut rand::thread_rng();
        match hands {
            Hands::TwoHanded(main_tool) => {
                let tool = main_tool.try_to_item(config_asset.unwrap_or("??"), rng);
                if let Some(tool) = tool {
                    self = self.with_main_tool(tool);
                }
            },
            Hands::Paired(tool) => {
                //FIXME: very stupid code, which just tries same item two times
                //figure out reasonable way to clone item
                let main_tool = tool.try_to_item(config_asset.unwrap_or("??"), rng);
                let second_tool = tool.try_to_item(config_asset.unwrap_or("??"), rng);
                if let Some(main_tool) = main_tool {
                    self = self.with_main_tool(main_tool);
                }
                if let Some(second_tool) = second_tool {
                    self = self.with_second_tool(second_tool);
                }
            },
            Hands::Mix { mainhand, offhand } => {
                let main_tool = mainhand.try_to_item(config_asset.unwrap_or("??"), rng);
                let second_tool = offhand.try_to_item(config_asset.unwrap_or("??"), rng);
                if let Some(main_tool) = main_tool {
                    self = self.with_main_tool(main_tool);
                }
                if let Some(second_tool) = second_tool {
                    self = self.with_second_tool(second_tool);
                }
            },
            Hands::Uninit => {},
        }

        for field in meta {
            match field {
                Meta::LoadoutAsset(asset) => {
                    self = self.with_loadout_asset(asset);
                },
                Meta::SkillSetAsset(asset) => {
                    self = self.with_skillset_asset(asset);
                },
            }
        }

        self
    }

    pub fn do_if(mut self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond {
            self = f(self);
        }
        self
    }

    pub fn into_waypoint(mut self) -> Self {
        self.is_waypoint = true;
        self
    }

    pub fn into_giant(mut self) -> Self {
        self.is_giant = true;
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_agency(mut self, agency: bool) -> Self {
        self.has_agency = agency;
        self
    }

    pub fn with_agent_mark(mut self, agent_mark: agent::Mark) -> Self {
        self.agent_mark = Some(agent_mark);
        self
    }

    pub fn with_main_tool(mut self, main_tool: Item) -> Self {
        self.main_tool = Some(main_tool);
        self
    }

    pub fn with_second_tool(mut self, second_tool: Item) -> Self {
        self.second_tool = Some(second_tool);
        self
    }

    pub fn with_loot_drop(mut self, loot_drop: Item) -> Self {
        self.loot_drop = Some(loot_drop);
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_level(mut self, level: u16) -> Self {
        self.level = Some(level);
        self
    }

    pub fn with_loadout_asset(mut self, asset: String) -> Self {
        self.loadout_asset = Some(asset);
        self
    }

    pub fn with_lazy_loadout(
        mut self,
        creator: fn(LoadoutBuilder, Option<&trade::SiteInformation>) -> LoadoutBuilder,
    ) -> Self {
        self.make_loadout = Some(creator);
        self
    }

    pub fn with_skillset_asset(mut self, asset: String) -> Self {
        self.skillset_asset = Some(asset);
        self
    }

    pub fn with_automatic_name(mut self) -> Self {
        let npc_names = NPC_NAMES.read();
        self.name = match &self.body {
            Body::Humanoid(body) => Some(get_npc_name(&npc_names.humanoid, body.species)),
            Body::QuadrupedMedium(body) => {
                Some(get_npc_name(&npc_names.quadruped_medium, body.species))
            },
            Body::BirdMedium(body) => Some(get_npc_name(&npc_names.bird_medium, body.species)),
            Body::BirdLarge(body) => Some(get_npc_name(&npc_names.bird_large, body.species)),
            Body::FishSmall(body) => Some(get_npc_name(&npc_names.fish_small, body.species)),
            Body::FishMedium(body) => Some(get_npc_name(&npc_names.fish_medium, body.species)),
            Body::Theropod(body) => Some(get_npc_name(&npc_names.theropod, body.species)),
            Body::QuadrupedSmall(body) => {
                Some(get_npc_name(&npc_names.quadruped_small, body.species))
            },
            Body::Dragon(body) => Some(get_npc_name(&npc_names.dragon, body.species)),
            Body::QuadrupedLow(body) => Some(get_npc_name(&npc_names.quadruped_low, body.species)),
            Body::Golem(body) => Some(get_npc_name(&npc_names.golem, body.species)),
            Body::BipedLarge(body) => Some(get_npc_name(&npc_names.biped_large, body.species)),
            _ => None,
        }
        .map(|s| {
            if self.is_giant {
                format!("Giant {}", s)
            } else {
                s.to_string()
            }
        });
        self
    }

    // map contains price+amount
    pub fn with_economy(mut self, e: &SiteInformation) -> Self {
        self.trading_information = Some(e.clone());
        self
    }
}

#[derive(Default)]
pub struct ChunkSupplement {
    pub entities: Vec<EntityInfo>,
}

impl ChunkSupplement {
    pub fn add_entity(&mut self, entity: EntityInfo) { self.entities.push(entity); }
}

pub fn get_npc_name<
    'a,
    Species,
    SpeciesData: for<'b> core::ops::Index<&'b Species, Output = npc::SpeciesNames>,
>(
    body_data: &'a comp::BodyData<npc::BodyNames, SpeciesData>,
    species: Species,
) -> &'a str {
    &body_data.species[&species].generic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{comp::inventory::slot::EquipSlot, SkillSetBuilder};
    use hashbrown::HashMap;

    #[derive(Debug, Eq, Hash, PartialEq)]
    enum MetaId {
        LoadoutAsset,
        SkillSetAsset,
    }

    impl Meta {
        fn id(&self) -> MetaId {
            match self {
                Meta::LoadoutAsset(_) => MetaId::LoadoutAsset,
                Meta::SkillSetAsset(_) => MetaId::SkillSetAsset,
            }
        }
    }

    fn validate_hands(hands: Hands, _config_asset: &str) {
        match hands {
            Hands::TwoHanded(main_tool) => {
                main_tool.validate(EquipSlot::ActiveMainhand);
            },
            Hands::Paired(tool) => {
                tool.validate(EquipSlot::ActiveMainhand);
                tool.validate(EquipSlot::ActiveOffhand);
            },
            Hands::Mix { mainhand, offhand } => {
                mainhand.validate(EquipSlot::ActiveMainhand);
                offhand.validate(EquipSlot::ActiveOffhand);
            },
            Hands::Uninit => {},
        }
    }

    fn validate_body_and_name(body: BodyBuilder, name: NameKind, config_asset: &str) {
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
            BodyBuilder::Uninit => {
                if let NameKind::Automatic = name {
                    // there is a big chance to call automatic name
                    // when body is yet undefined
                    //
                    // use .with_automatic_name() in code explicitly
                    panic!("Used Automatic name with Uninit body in {}", config_asset);
                }
            },
            BodyBuilder::Exact { .. } => {},
        }
    }

    fn validate_loot(loot: LootKind, config_asset: &str) {
        match loot {
            LootKind::Item(asset) => {
                if let Err(e) = Item::new_from_asset(&asset) {
                    panic!(
                        "Unable to parse loot item ({}) in {}. Err: {:?}",
                        asset, config_asset, e
                    );
                }
            },
            LootKind::LootTable(asset) => {
                // we need to just load it check if it exists,
                // because all loot tables are tested in Lottery module
                if let Err(e) = Lottery::<LootSpec<String>>::load(&asset) {
                    panic!(
                        "Unable to parse loot table ({}) in {}. Err: {:?}",
                        asset, config_asset, e
                    );
                }
            },
            LootKind::Uninit => {},
        }
    }

    fn validate_meta(meta: Vec<Meta>, config_asset: &str) {
        let mut meta_counter = HashMap::new();
        for field in meta {
            meta_counter
                .entry(field.id())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            match field {
                Meta::LoadoutAsset(asset) => {
                    let rng = &mut rand::thread_rng();
                    let builder = LoadoutBuilder::empty();
                    // we need to just load it check if it exists,
                    // because all loadouts are tested in LoadoutBuilder module
                    std::mem::drop(builder.with_asset_expect(&asset, rng));
                },
                Meta::SkillSetAsset(asset) => {
                    std::mem::drop(SkillSetBuilder::from_asset_expect(&asset));
                },
            }
        }
        for (meta_id, counter) in meta_counter {
            if counter > 1 {
                panic!("Duplicate {:?} in {}", meta_id, config_asset);
            }
        }
    }

    #[test]
    fn test_all_entity_assets() {
        // Get list of entity configs, load everything, validate content.
        let entity_configs =
            try_all_entity_configs().expect("Failed to access entity configs directory");
        for config_asset in entity_configs {
            // print asset name so we don't need to find errors everywhere
            // it'll be ignored by default so you'll see it only in case of errors
            //
            // TODO:
            // 1) Add try_validate() for loadout_builder::ItemSpec which will return
            // Result and we will happily panic in validate_hands() with name of
            // config_asset.
            // 2) Add try_from_asset() for LoadoutBuilder and
            // SkillSet builder which will return Result and we will happily
            // panic in validate_meta() with the name of config_asset
            println!("{}:", &config_asset);

            let EntityConfig {
                hands,
                body,
                name,
                loot,
                meta,
                alignment: _alignment, // can't fail if serialized, it's a boring enum
            } = EntityConfig::from_asset_expect(&config_asset);

            validate_hands(hands, &config_asset);
            validate_body_and_name(body, name, &config_asset);
            validate_loot(loot, &config_asset);
            validate_meta(meta, &config_asset);
        }
    }
}
