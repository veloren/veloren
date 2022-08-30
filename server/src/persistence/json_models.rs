use common::comp;
use common_base::dev_panic;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::string::ToString;
use vek::{Vec2, Vec3};

#[derive(Serialize, Deserialize)]
pub struct HumanoidBody {
    pub species: u8,
    pub body_type: u8,
    pub hair_style: u8,
    pub beard: u8,
    pub eyes: u8,
    pub accessory: u8,
    pub hair_color: u8,
    pub skin: u8,
    pub eye_color: u8,
}

impl From<&comp::humanoid::Body> for HumanoidBody {
    fn from(body: &comp::humanoid::Body) -> Self {
        HumanoidBody {
            species: body.species as u8,
            body_type: body.body_type as u8,
            hair_style: body.hair_style,
            beard: body.beard,
            eyes: body.eyes,
            accessory: body.accessory,
            hair_color: body.hair_color,
            skin: body.skin,
            eye_color: body.eye_color,
        }
    }
}

/// A serializable model used to represent a generic Body. Since all variants
/// of Body except Humanoid (currently) have the same struct layout, a single
/// struct is used for persistence conversions.
#[derive(Serialize, Deserialize)]
pub struct GenericBody {
    pub species: String,
    pub body_type: String,
}

macro_rules! generic_body_from_impl {
    ($body_type:ty) => {
        impl From<&$body_type> for GenericBody {
            fn from(body: &$body_type) -> Self {
                GenericBody {
                    species: body.species.to_string(),
                    body_type: body.body_type.to_string(),
                }
            }
        }
    };
}

generic_body_from_impl!(comp::quadruped_low::Body);
generic_body_from_impl!(comp::quadruped_medium::Body);
generic_body_from_impl!(comp::quadruped_small::Body);
generic_body_from_impl!(comp::bird_medium::Body);

#[derive(Serialize, Deserialize)]
pub struct CharacterPosition {
    pub waypoint: Option<Vec3<f32>>,
    pub map_marker: Option<Vec2<i32>>,
}

pub fn skill_group_to_db_string(skill_group: comp::skillset::SkillGroupKind) -> String {
    use comp::{item::tool::ToolKind, skillset::SkillGroupKind::*};
    let skill_group_string = match skill_group {
        General => "General",
        Weapon(ToolKind::Sword) => "Weapon Sword",
        Weapon(ToolKind::Axe) => "Weapon Axe",
        Weapon(ToolKind::Hammer) => "Weapon Hammer",
        Weapon(ToolKind::Bow) => "Weapon Bow",
        Weapon(ToolKind::Staff) => "Weapon Staff",
        Weapon(ToolKind::Sceptre) => "Weapon Sceptre",
        Weapon(ToolKind::Pick) => "Weapon Pick",
        Weapon(ToolKind::Dagger)
        | Weapon(ToolKind::Shield)
        | Weapon(ToolKind::Spear)
        | Weapon(ToolKind::Blowgun)
        | Weapon(ToolKind::Debug)
        | Weapon(ToolKind::Farming)
        | Weapon(ToolKind::Instrument)
        | Weapon(ToolKind::Empty)
        | Weapon(ToolKind::Natural) => panic!(
            "Tried to add unsupported skill group to database: {:?}",
            skill_group
        ),
    };
    skill_group_string.to_string()
}

pub fn db_string_to_skill_group(skill_group_string: &str) -> comp::skillset::SkillGroupKind {
    use comp::{item::tool::ToolKind, skillset::SkillGroupKind::*};
    match skill_group_string {
        "General" => General,
        "Weapon Sword" => Weapon(ToolKind::Sword),
        "Weapon Axe" => Weapon(ToolKind::Axe),
        "Weapon Hammer" => Weapon(ToolKind::Hammer),
        "Weapon Bow" => Weapon(ToolKind::Bow),
        "Weapon Staff" => Weapon(ToolKind::Staff),
        "Weapon Sceptre" => Weapon(ToolKind::Sceptre),
        "Weapon Pick" => Weapon(ToolKind::Pick),

        _ => panic!(
            "Tried to convert an unsupported string from the database: {}",
            skill_group_string
        ),
    }
}

#[derive(Serialize, Deserialize)]
pub struct DatabaseAbilitySet {
    mainhand: String,
    offhand: String,
    abilities: Vec<String>,
}

fn aux_ability_to_string(ability: comp::ability::AuxiliaryAbility) -> String {
    use common::comp::ability::AuxiliaryAbility;
    match ability {
        AuxiliaryAbility::MainWeapon(index) => format!("Main Weapon:index:{}", index),
        AuxiliaryAbility::OffWeapon(index) => format!("Off Weapon:index:{}", index),
        AuxiliaryAbility::Empty => String::from("Empty"),
    }
}

fn aux_ability_from_string(ability: &str) -> comp::ability::AuxiliaryAbility {
    use common::comp::ability::AuxiliaryAbility;
    let mut parts = ability.split(":index:");
    match parts.next() {
        Some("Main Weapon") => match parts
            .next()
            .map(|index| index.parse::<usize>().map_err(|_| index))
        {
            Some(Ok(index)) => AuxiliaryAbility::MainWeapon(index),
            Some(Err(error)) => {
                dev_panic!(format!(
                    "Conversion from database to ability set failed. Unable to parse index for \
                     mainhand abilities: {}",
                    error
                ));
                AuxiliaryAbility::Empty
            },
            None => {
                dev_panic!(String::from(
                    "Conversion from database to ability set failed. Unable to find an index for \
                     mainhand abilities"
                ));
                AuxiliaryAbility::Empty
            },
        },
        Some("Off Weapon") => match parts
            .next()
            .map(|index| index.parse::<usize>().map_err(|_| index))
        {
            Some(Ok(index)) => AuxiliaryAbility::OffWeapon(index),
            Some(Err(error)) => {
                dev_panic!(format!(
                    "Conversion from database to ability set failed. Unable to parse index for \
                     offhand abilities: {}",
                    error
                ));
                AuxiliaryAbility::Empty
            },
            None => {
                dev_panic!(String::from(
                    "Conversion from database to ability set failed. Unable to find an index for \
                     offhand abilities"
                ));
                AuxiliaryAbility::Empty
            },
        },
        Some("Empty") => AuxiliaryAbility::Empty,
        unknown => {
            dev_panic!(format!(
                "Conversion from database to ability set failed. Unknown auxiliary ability: {:#?}",
                unknown
            ));
            AuxiliaryAbility::Empty
        },
    }
}

fn tool_kind_to_string(tool: Option<comp::item::tool::ToolKind>) -> String {
    use common::comp::item::tool::ToolKind::*;
    String::from(match tool {
        Some(Sword) => "Sword",
        Some(Axe) => "Axe",
        Some(Hammer) => "Hammer",
        Some(Bow) => "Bow",
        Some(Staff) => "Staff",
        Some(Sceptre) => "Sceptre",
        Some(Dagger) => "Dagger",
        Some(Shield) => "Shield",
        Some(Spear) => "Spear",
        Some(Blowgun) => "Blowgun",
        Some(Pick) => "Pick",

        // Toolkinds that are not anticipated to have many active abilities (if any at all)
        Some(Farming) => "Farming",
        Some(Debug) => "Debug",
        Some(Natural) => "Natural",
        Some(Instrument) => "Instrument",
        Some(Empty) => "Empty",
        None => "None",
    })
}

fn tool_kind_from_string(tool: String) -> Option<comp::item::tool::ToolKind> {
    use common::comp::item::tool::ToolKind::*;
    match tool.as_str() {
        "Sword" => Some(Sword),
        "Axe" => Some(Axe),
        "Hammer" => Some(Hammer),
        "Bow" => Some(Bow),
        "Staff" => Some(Staff),
        "Sceptre" => Some(Sceptre),
        "Dagger" => Some(Dagger),
        "Shield" => Some(Shield),
        "Spear" => Some(Spear),
        "Blowgun" => Some(Blowgun),
        "Pick" => Some(Pick),
        "Farming" => Some(Farming),
        "Debug" => Some(Debug),
        "Natural" => Some(Natural),
        "Empty" => Some(Empty),
        "None" => None,
        unknown => {
            dev_panic!(format!(
                "Conversion from database to ability set failed. Unknown toolkind: {:#?}",
                unknown
            ));
            None
        },
    }
}

pub fn active_abilities_to_db_model(
    active_abilities: &comp::ability::ActiveAbilities,
) -> Vec<DatabaseAbilitySet> {
    active_abilities
        .auxiliary_sets
        .iter()
        .map(|((mainhand, offhand), abilities)| DatabaseAbilitySet {
            mainhand: tool_kind_to_string(*mainhand),
            offhand: tool_kind_to_string(*offhand),
            abilities: abilities
                .iter()
                .map(|ability| aux_ability_to_string(*ability))
                .collect(),
        })
        .collect::<Vec<_>>()
}

pub fn active_abilities_from_db_model(
    ability_sets: Vec<DatabaseAbilitySet>,
) -> comp::ability::ActiveAbilities {
    let ability_sets = ability_sets
        .into_iter()
        .map(
            |DatabaseAbilitySet {
                 mainhand,
                 offhand,
                 abilities,
             }| {
                let mut auxiliary_abilities =
                    [comp::ability::AuxiliaryAbility::Empty; comp::ability::MAX_ABILITIES];
                for (empty, ability) in auxiliary_abilities.iter_mut().zip(abilities.into_iter()) {
                    *empty = aux_ability_from_string(&ability);
                }
                (
                    (
                        tool_kind_from_string(mainhand),
                        tool_kind_from_string(offhand),
                    ),
                    auxiliary_abilities,
                )
            },
        )
        .collect::<HashMap<_, _>>();
    comp::ability::ActiveAbilities::new(ability_sets)
}
