use common::comp;
use serde::{Deserialize, Serialize};
use std::string::ToString;
use tracing::error;
use vek::Vec3;

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

#[derive(Serialize, Deserialize)]
pub struct CharacterPosition {
    pub waypoint: Vec3<f32>,
}

pub fn skill_to_db_string(skill: comp::skills::Skill) -> String {
    use comp::{
        item::tool::ToolKind,
        skills::{
            AxeSkill, BowSkill, ClimbSkill, GeneralSkill, HammerSkill, MiningSkill, RollSkill,
            SceptreSkill, Skill::*, StaffSkill, SwimSkill, SwordSkill,
        },
        skillset::SkillGroupKind,
    };
    let skill_string = match skill {
        General(GeneralSkill::HealthIncrease) => "General HealthIncrease",
        General(GeneralSkill::EnergyIncrease) => "General EnergyIncrease",
        Sword(SwordSkill::InterruptingAttacks) => "Sword InterruptingAttacks",
        Sword(SwordSkill::TsCombo) => "Sword TsCombo",
        Sword(SwordSkill::TsDamage) => "Sword TsDamage",
        Sword(SwordSkill::TsRegen) => "Sword TsRegen",
        Sword(SwordSkill::TsSpeed) => "Sword TsSpeed",
        Sword(SwordSkill::DCost) => "Sword DCost",
        Sword(SwordSkill::DDrain) => "Sword DDrain",
        Sword(SwordSkill::DDamage) => "Sword DDamage",
        Sword(SwordSkill::DScaling) => "Sword DScaling",
        Sword(SwordSkill::DSpeed) => "Sword DSpeed",
        Sword(SwordSkill::DInfinite) => "Sword DInfinite",
        Sword(SwordSkill::UnlockSpin) => "Sword UnlockSpin",
        Sword(SwordSkill::SDamage) => "Sword SDamage",
        Sword(SwordSkill::SSpeed) => "Sword SSpeed",
        Sword(SwordSkill::SCost) => "Sword SCost",
        Sword(SwordSkill::SSpins) => "Sword SSpins",
        Axe(AxeSkill::DsCombo) => "Axe DsCombo",
        Axe(AxeSkill::DsDamage) => "Axe DsDamage",
        Axe(AxeSkill::DsSpeed) => "Axe DsSpeed",
        Axe(AxeSkill::DsRegen) => "Axe DsRegen",
        Axe(AxeSkill::SInfinite) => "Axe SInfinite",
        Axe(AxeSkill::SHelicopter) => "Axe SHelicopter",
        Axe(AxeSkill::SDamage) => "Axe SDamage",
        Axe(AxeSkill::SSpeed) => "Axe SSpeed",
        Axe(AxeSkill::SCost) => "Axe SCost",
        Axe(AxeSkill::UnlockLeap) => "Axe UnlockLeap",
        Axe(AxeSkill::LDamage) => "Axe LDamage",
        Axe(AxeSkill::LKnockback) => "Axe LKnockback",
        Axe(AxeSkill::LCost) => "Axe LCost",
        Axe(AxeSkill::LDistance) => "Axe LDistance",
        Hammer(HammerSkill::SsKnockback) => "Hammer SsKnockback",
        Hammer(HammerSkill::SsDamage) => "Hammer SsDamage",
        Hammer(HammerSkill::SsSpeed) => "Hammer SsSpeed",
        Hammer(HammerSkill::SsRegen) => "Hammer SsRegen",
        Hammer(HammerSkill::CDamage) => "Hammer CDamage",
        Hammer(HammerSkill::CKnockback) => "Hammer CKnockback",
        Hammer(HammerSkill::CDrain) => "Hammer CDrain",
        Hammer(HammerSkill::CSpeed) => "Hammer CSpeed",
        Hammer(HammerSkill::UnlockLeap) => "Hammer UnlockLeap",
        Hammer(HammerSkill::LDamage) => "Hammer LDamage",
        Hammer(HammerSkill::LCost) => "Hammer LCost",
        Hammer(HammerSkill::LDistance) => "Hammer LDistance",
        Hammer(HammerSkill::LKnockback) => "Hammer LKnockback",
        Hammer(HammerSkill::LRange) => "Hammer LRange",
        Bow(BowSkill::ProjSpeed) => "Bow ProjSpeed",
        Bow(BowSkill::CDamage) => "Bow CDamage",
        Bow(BowSkill::CRegen) => "Bow CRegen",
        Bow(BowSkill::CKnockback) => "Bow CKnockback",
        Bow(BowSkill::CSpeed) => "Bow CSpeed",
        Bow(BowSkill::CMove) => "Bow CMove",
        Bow(BowSkill::RDamage) => "Bow RDamage",
        Bow(BowSkill::RCost) => "Bow RCost",
        Bow(BowSkill::RSpeed) => "Bow RSpeed",
        Bow(BowSkill::UnlockShotgun) => "Bow UnlockShotgun",
        Bow(BowSkill::SDamage) => "Bow SDamage",
        Bow(BowSkill::SCost) => "Bow SCost",
        Bow(BowSkill::SArrows) => "Bow SArrows",
        Bow(BowSkill::SSpread) => "Bow SSpread",
        Staff(StaffSkill::BDamage) => "Staff BDamage",
        Staff(StaffSkill::BRegen) => "Staff BRegen",
        Staff(StaffSkill::BRadius) => "Staff BRadius",
        Staff(StaffSkill::FDamage) => "Staff FDamage",
        Staff(StaffSkill::FRange) => "Staff FRange",
        Staff(StaffSkill::FDrain) => "Staff FDrain",
        Staff(StaffSkill::FVelocity) => "Staff FVelocity",
        Staff(StaffSkill::UnlockShockwave) => "Staff UnlockShockwave",
        Staff(StaffSkill::SDamage) => "Staff SDamage",
        Staff(StaffSkill::SKnockback) => "Staff SKnockback",
        Staff(StaffSkill::SRange) => "Staff SRange",
        Staff(StaffSkill::SCost) => "Staff SCost",
        Sceptre(SceptreSkill::LDamage) => "Sceptre LDamage",
        Sceptre(SceptreSkill::LRange) => "Sceptre LRange",
        Sceptre(SceptreSkill::LLifesteal) => "Sceptre LLifesteal",
        Sceptre(SceptreSkill::LRegen) => "Sceptre LRegen",
        Sceptre(SceptreSkill::HHeal) => "Sceptre HHeal",
        Sceptre(SceptreSkill::HDuration) => "Sceptre HDuration",
        Sceptre(SceptreSkill::HRange) => "Sceptre HRange",
        Sceptre(SceptreSkill::HCost) => "Sceptre HCost",
        Sceptre(SceptreSkill::UnlockAura) => "Sceptre UnlockAura",
        Sceptre(SceptreSkill::AStrength) => "Sceptre AStrength",
        Sceptre(SceptreSkill::ADuration) => "Sceptre ADuration",
        Sceptre(SceptreSkill::ARange) => "Sceptre ARange",
        Sceptre(SceptreSkill::ACost) => "Sceptre ACost",
        Roll(RollSkill::Cost) => "Roll Cost",
        Roll(RollSkill::Strength) => "Roll Strength",
        Roll(RollSkill::Duration) => "Roll Duration",
        Climb(ClimbSkill::Cost) => "Climb Cost",
        Climb(ClimbSkill::Speed) => "Climb Speed",
        Swim(SwimSkill::Speed) => "Swim Speed",
        Pick(MiningSkill::Speed) => "Pick Speed",
        Pick(MiningSkill::OreGain) => "Pick OreGain",
        Pick(MiningSkill::GemGain) => "Pick GemGain",
        UnlockGroup(SkillGroupKind::Weapon(ToolKind::Sword)) => "Unlock Weapon Sword",
        UnlockGroup(SkillGroupKind::Weapon(ToolKind::Axe)) => "Unlock Weapon Axe",
        UnlockGroup(SkillGroupKind::Weapon(ToolKind::Hammer)) => "Unlock Weapon Hammer",
        UnlockGroup(SkillGroupKind::Weapon(ToolKind::Bow)) => "Unlock Weapon Bow",
        UnlockGroup(SkillGroupKind::Weapon(ToolKind::Staff)) => "Unlock Weapon Staff",
        UnlockGroup(SkillGroupKind::Weapon(ToolKind::Sceptre)) => "Unlock Weapon Sceptre",
        UnlockGroup(SkillGroupKind::Weapon(ToolKind::Dagger))
        | UnlockGroup(SkillGroupKind::Weapon(ToolKind::Shield))
        | UnlockGroup(SkillGroupKind::Weapon(ToolKind::Spear))
        | UnlockGroup(SkillGroupKind::Weapon(ToolKind::Debug))
        | UnlockGroup(SkillGroupKind::Weapon(ToolKind::Farming))
        | UnlockGroup(SkillGroupKind::Weapon(ToolKind::Pick))
        | UnlockGroup(SkillGroupKind::Weapon(ToolKind::Empty))
        | UnlockGroup(SkillGroupKind::Weapon(ToolKind::Natural))
        | UnlockGroup(SkillGroupKind::General) => {
            error!("Tried to add unsupported skill to database: {:?}", skill);
            "Invalid Skill"
        },
    };
    skill_string.to_string()
}

pub fn db_string_to_skill(skill_string: &str) -> Option<comp::skills::Skill> {
    use comp::{
        item::tool::ToolKind,
        skills::{
            AxeSkill, BowSkill, ClimbSkill, GeneralSkill, HammerSkill, MiningSkill, RollSkill,
            SceptreSkill, Skill::*, StaffSkill, SwimSkill, SwordSkill,
        },
        skillset::SkillGroupKind,
    };
    match skill_string {
        "General HealthIncrease" => Some(General(GeneralSkill::HealthIncrease)),
        "General EnergyIncrease" => Some(General(GeneralSkill::EnergyIncrease)),
        "Sword InterruptingAttacks" => Some(Sword(SwordSkill::InterruptingAttacks)),
        "Sword TsCombo" => Some(Sword(SwordSkill::TsCombo)),
        "Sword TsDamage" => Some(Sword(SwordSkill::TsDamage)),
        "Sword TsRegen" => Some(Sword(SwordSkill::TsRegen)),
        "Sword TsSpeed" => Some(Sword(SwordSkill::TsSpeed)),
        "Sword DCost" => Some(Sword(SwordSkill::DCost)),
        "Sword DDrain" => Some(Sword(SwordSkill::DDrain)),
        "Sword DDamage" => Some(Sword(SwordSkill::DDamage)),
        "Sword DScaling" => Some(Sword(SwordSkill::DScaling)),
        "Sword DSpeed" => Some(Sword(SwordSkill::DSpeed)),
        "Sword DInfinite" => Some(Sword(SwordSkill::DInfinite)),
        "Sword UnlockSpin" => Some(Sword(SwordSkill::UnlockSpin)),
        "Sword SDamage" => Some(Sword(SwordSkill::SDamage)),
        "Sword SSpeed" => Some(Sword(SwordSkill::SSpeed)),
        "Sword SCost" => Some(Sword(SwordSkill::SCost)),
        "Sword SSpins" => Some(Sword(SwordSkill::SSpins)),
        "Axe DsCombo" => Some(Axe(AxeSkill::DsCombo)),
        "Axe DsDamage" => Some(Axe(AxeSkill::DsDamage)),
        "Axe DsSpeed" => Some(Axe(AxeSkill::DsSpeed)),
        "Axe DsRegen" => Some(Axe(AxeSkill::DsRegen)),
        "Axe SInfinite" => Some(Axe(AxeSkill::SInfinite)),
        "Axe SHelicopter" => Some(Axe(AxeSkill::SHelicopter)),
        "Axe SDamage" => Some(Axe(AxeSkill::SDamage)),
        "Axe SSpeed" => Some(Axe(AxeSkill::SSpeed)),
        "Axe SCost" => Some(Axe(AxeSkill::SCost)),
        "Axe UnlockLeap" => Some(Axe(AxeSkill::UnlockLeap)),
        "Axe LDamage" => Some(Axe(AxeSkill::LDamage)),
        "Axe LKnockback" => Some(Axe(AxeSkill::LKnockback)),
        "Axe LCost" => Some(Axe(AxeSkill::LCost)),
        "Axe LDistance" => Some(Axe(AxeSkill::LDistance)),
        "Hammer SsKnockback" => Some(Hammer(HammerSkill::SsKnockback)),
        "Hammer SsDamage" => Some(Hammer(HammerSkill::SsDamage)),
        "Hammer SsSpeed" => Some(Hammer(HammerSkill::SsSpeed)),
        "Hammer SsRegen" => Some(Hammer(HammerSkill::SsRegen)),
        "Hammer CDamage" => Some(Hammer(HammerSkill::CDamage)),
        "Hammer CKnockback" => Some(Hammer(HammerSkill::CKnockback)),
        "Hammer CDrain" => Some(Hammer(HammerSkill::CDrain)),
        "Hammer CSpeed" => Some(Hammer(HammerSkill::CSpeed)),
        "Hammer UnlockLeap" => Some(Hammer(HammerSkill::UnlockLeap)),
        "Hammer LDamage" => Some(Hammer(HammerSkill::LDamage)),
        "Hammer LCost" => Some(Hammer(HammerSkill::LCost)),
        "Hammer LDistance" => Some(Hammer(HammerSkill::LDistance)),
        "Hammer LKnockback" => Some(Hammer(HammerSkill::LKnockback)),
        "Hammer LRange" => Some(Hammer(HammerSkill::LRange)),
        "Bow ProjSpeed" => Some(Bow(BowSkill::ProjSpeed)),
        "Bow CDamage" => Some(Bow(BowSkill::CDamage)),
        "Bow CRegen" => Some(Bow(BowSkill::CRegen)),
        "Bow CKnockback" => Some(Bow(BowSkill::CKnockback)),
        "Bow CSpeed" => Some(Bow(BowSkill::CSpeed)),
        "Bow CMove" => Some(Bow(BowSkill::CMove)),
        "Bow RDamage" => Some(Bow(BowSkill::RDamage)),
        "Bow RCost" => Some(Bow(BowSkill::RCost)),
        "Bow RSpeed" => Some(Bow(BowSkill::RSpeed)),
        "Bow UnlockShotgun" => Some(Bow(BowSkill::UnlockShotgun)),
        "Bow SDamage" => Some(Bow(BowSkill::SDamage)),
        "Bow SCost" => Some(Bow(BowSkill::SCost)),
        "Bow SArrows" => Some(Bow(BowSkill::SArrows)),
        "Bow SSpread" => Some(Bow(BowSkill::SSpread)),
        "Staff BDamage" => Some(Staff(StaffSkill::BDamage)),
        "Staff BRegen" => Some(Staff(StaffSkill::BRegen)),
        "Staff BRadius" => Some(Staff(StaffSkill::BRadius)),
        "Staff FDamage" => Some(Staff(StaffSkill::FDamage)),
        "Staff FRange" => Some(Staff(StaffSkill::FRange)),
        "Staff FDrain" => Some(Staff(StaffSkill::FDrain)),
        "Staff FVelocity" => Some(Staff(StaffSkill::FVelocity)),
        "Staff UnlockShockwave" => Some(Staff(StaffSkill::UnlockShockwave)),
        "Staff SDamage" => Some(Staff(StaffSkill::SDamage)),
        "Staff SKnockback" => Some(Staff(StaffSkill::SKnockback)),
        "Staff SRange" => Some(Staff(StaffSkill::SRange)),
        "Staff SCost" => Some(Staff(StaffSkill::SCost)),
        "Sceptre LDamage" => Some(Sceptre(SceptreSkill::LDamage)),
        "Sceptre LRange" => Some(Sceptre(SceptreSkill::LRange)),
        "Sceptre LLifesteal" => Some(Sceptre(SceptreSkill::LLifesteal)),
        "Sceptre LRegen" => Some(Sceptre(SceptreSkill::LRegen)),
        "Sceptre HHeal" => Some(Sceptre(SceptreSkill::HHeal)),
        "Sceptre HDuration" => Some(Sceptre(SceptreSkill::HDuration)),
        "Sceptre HRange" => Some(Sceptre(SceptreSkill::HRange)),
        "Sceptre HCost" => Some(Sceptre(SceptreSkill::HCost)),
        "Sceptre UnlockAura" => Some(Sceptre(SceptreSkill::UnlockAura)),
        "Sceptre AStrength" => Some(Sceptre(SceptreSkill::AStrength)),
        "Sceptre ADuration" => Some(Sceptre(SceptreSkill::ADuration)),
        "Sceptre ARange" => Some(Sceptre(SceptreSkill::ARange)),
        "Sceptre ACost" => Some(Sceptre(SceptreSkill::ACost)),
        "Roll Cost" => Some(Roll(RollSkill::Cost)),
        "Roll Strength" => Some(Roll(RollSkill::Strength)),
        "Roll Duration" => Some(Roll(RollSkill::Duration)),
        "Climb Cost" => Some(Climb(ClimbSkill::Cost)),
        "Climb Speed" => Some(Climb(ClimbSkill::Speed)),
        "Swim Speed" => Some(Swim(SwimSkill::Speed)),
        "Pick Speed" => Some(Pick(MiningSkill::Speed)),
        "Pick GemGain" => Some(Pick(MiningSkill::GemGain)),
        "Pick OreGain" => Some(Pick(MiningSkill::OreGain)),
        "Unlock Weapon Sword" => Some(UnlockGroup(SkillGroupKind::Weapon(ToolKind::Sword))),
        "Unlock Weapon Axe" => Some(UnlockGroup(SkillGroupKind::Weapon(ToolKind::Axe))),
        "Unlock Weapon Hammer" => Some(UnlockGroup(SkillGroupKind::Weapon(ToolKind::Hammer))),
        "Unlock Weapon Bow" => Some(UnlockGroup(SkillGroupKind::Weapon(ToolKind::Bow))),
        "Unlock Weapon Staff" => Some(UnlockGroup(SkillGroupKind::Weapon(ToolKind::Staff))),
        "Unlock Weapon Sceptre" => Some(UnlockGroup(SkillGroupKind::Weapon(ToolKind::Sceptre))),
        _ => {
            error!(
                "Tried to convert an unsupported string from the database: {}",
                skill_string
            );
            None
        },
    }
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
        | Weapon(ToolKind::Debug)
        | Weapon(ToolKind::Farming)
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
