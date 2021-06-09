use common::comp;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize)]
pub struct CharacterPosition {
    pub waypoint: Vec3<f32>,
}

pub fn skill_to_db_string(skill: comp::skills::Skill) -> String {
    use comp::{
        item::tool::ToolKind,
        skills::{
            AxeSkill, BowSkill, ClimbSkill, GeneralSkill, HammerSkill, MiningSkill, RollSkill,
            SceptreSkill, Skill::*, SkillGroupKind, StaffSkill, SwimSkill, SwordSkill,
        },
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
        Sceptre(SceptreSkill::HCost) => "Sceptre HCost",
        Sceptre(SceptreSkill::HRange) => "Sceptre HRange",
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
            panic!("Tried to add unsupported skill to database: {:?}", skill)
        },
    };
    skill_string.to_string()
}

pub fn db_string_to_skill(skill_string: &str) -> comp::skills::Skill {
    use comp::{
        item::tool::ToolKind,
        skills::{
            AxeSkill, BowSkill, ClimbSkill, GeneralSkill, HammerSkill, MiningSkill, RollSkill,
            SceptreSkill, Skill::*, SkillGroupKind, StaffSkill, SwimSkill, SwordSkill,
        },
    };
    match skill_string {
        "General HealthIncrease" => General(GeneralSkill::HealthIncrease),
        "General EnergyIncrease" => General(GeneralSkill::EnergyIncrease),
        "Sword InterruptingAttacks" => Sword(SwordSkill::InterruptingAttacks),
        "Sword TsCombo" => Sword(SwordSkill::TsCombo),
        "Sword TsDamage" => Sword(SwordSkill::TsDamage),
        "Sword TsRegen" => Sword(SwordSkill::TsRegen),
        "Sword TsSpeed" => Sword(SwordSkill::TsSpeed),
        "Sword DCost" => Sword(SwordSkill::DCost),
        "Sword DDrain" => Sword(SwordSkill::DDrain),
        "Sword DDamage" => Sword(SwordSkill::DDamage),
        "Sword DScaling" => Sword(SwordSkill::DScaling),
        "Sword DSpeed" => Sword(SwordSkill::DSpeed),
        "Sword DInfinite" => Sword(SwordSkill::DInfinite),
        "Sword UnlockSpin" => Sword(SwordSkill::UnlockSpin),
        "Sword SDamage" => Sword(SwordSkill::SDamage),
        "Sword SSpeed" => Sword(SwordSkill::SSpeed),
        "Sword SCost" => Sword(SwordSkill::SCost),
        "Sword SSpins" => Sword(SwordSkill::SSpins),
        "Axe DsCombo" => Axe(AxeSkill::DsCombo),
        "Axe DsDamage" => Axe(AxeSkill::DsDamage),
        "Axe DsSpeed" => Axe(AxeSkill::DsSpeed),
        "Axe DsRegen" => Axe(AxeSkill::DsRegen),
        "Axe SInfinite" => Axe(AxeSkill::SInfinite),
        "Axe SHelicopter" => Axe(AxeSkill::SHelicopter),
        "Axe SDamage" => Axe(AxeSkill::SDamage),
        "Axe SSpeed" => Axe(AxeSkill::SSpeed),
        "Axe SCost" => Axe(AxeSkill::SCost),
        "Axe UnlockLeap" => Axe(AxeSkill::UnlockLeap),
        "Axe LDamage" => Axe(AxeSkill::LDamage),
        "Axe LKnockback" => Axe(AxeSkill::LKnockback),
        "Axe LCost" => Axe(AxeSkill::LCost),
        "Axe LDistance" => Axe(AxeSkill::LDistance),
        "Hammer SsKnockback" => Hammer(HammerSkill::SsKnockback),
        "Hammer SsDamage" => Hammer(HammerSkill::SsDamage),
        "Hammer SsSpeed" => Hammer(HammerSkill::SsSpeed),
        "Hammer SsRegen" => Hammer(HammerSkill::SsRegen),
        "Hammer CDamage" => Hammer(HammerSkill::CDamage),
        "Hammer CKnockback" => Hammer(HammerSkill::CKnockback),
        "Hammer CDrain" => Hammer(HammerSkill::CDrain),
        "Hammer CSpeed" => Hammer(HammerSkill::CSpeed),
        "Hammer UnlockLeap" => Hammer(HammerSkill::UnlockLeap),
        "Hammer LDamage" => Hammer(HammerSkill::LDamage),
        "Hammer LCost" => Hammer(HammerSkill::LCost),
        "Hammer LDistance" => Hammer(HammerSkill::LDistance),
        "Hammer LKnockback" => Hammer(HammerSkill::LKnockback),
        "Hammer LRange" => Hammer(HammerSkill::LRange),
        "Bow ProjSpeed" => Bow(BowSkill::ProjSpeed),
        "Bow CDamage" => Bow(BowSkill::CDamage),
        "Bow CRegen" => Bow(BowSkill::CRegen),
        "Bow CKnockback" => Bow(BowSkill::CKnockback),
        "Bow CSpeed" => Bow(BowSkill::CSpeed),
        "Bow CMove" => Bow(BowSkill::CMove),
        "Bow RDamage" => Bow(BowSkill::RDamage),
        "Bow RCost" => Bow(BowSkill::RCost),
        "Bow RSpeed" => Bow(BowSkill::RSpeed),
        "Bow UnlockShotgun" => Bow(BowSkill::UnlockShotgun),
        "Bow SDamage" => Bow(BowSkill::SDamage),
        "Bow SCost" => Bow(BowSkill::SCost),
        "Bow SArrows" => Bow(BowSkill::SArrows),
        "Bow SSpread" => Bow(BowSkill::SSpread),
        "Staff BDamage" => Staff(StaffSkill::BDamage),
        "Staff BRegen" => Staff(StaffSkill::BRegen),
        "Staff BRadius" => Staff(StaffSkill::BRadius),
        "Staff FDamage" => Staff(StaffSkill::FDamage),
        "Staff FRange" => Staff(StaffSkill::FRange),
        "Staff FDrain" => Staff(StaffSkill::FDrain),
        "Staff FVelocity" => Staff(StaffSkill::FVelocity),
        "Staff UnlockShockwave" => Staff(StaffSkill::UnlockShockwave),
        "Staff SDamage" => Staff(StaffSkill::SDamage),
        "Staff SKnockback" => Staff(StaffSkill::SKnockback),
        "Staff SRange" => Staff(StaffSkill::SRange),
        "Staff SCost" => Staff(StaffSkill::SCost),
        "Sceptre LDamage" => Sceptre(SceptreSkill::LDamage),
        "Sceptre LRange" => Sceptre(SceptreSkill::LRange),
        "Sceptre LLifesteal" => Sceptre(SceptreSkill::LLifesteal),
        "Sceptre LRegen" => Sceptre(SceptreSkill::LRegen),
        "Sceptre HHeal" => Sceptre(SceptreSkill::HHeal),
        "Sceptre HCost" => Sceptre(SceptreSkill::HCost),
        "Sceptre HRange" => Sceptre(SceptreSkill::HRange),
        "Sceptre UnlockAura" => Sceptre(SceptreSkill::UnlockAura),
        "Sceptre AStrength" => Sceptre(SceptreSkill::AStrength),
        "Sceptre ADuration" => Sceptre(SceptreSkill::ADuration),
        "Sceptre ARange" => Sceptre(SceptreSkill::ARange),
        "Sceptre ACost" => Sceptre(SceptreSkill::ACost),
        "Roll Cost" => Roll(RollSkill::Cost),
        "Roll Strength" => Roll(RollSkill::Strength),
        "Roll Duration" => Roll(RollSkill::Duration),
        "Climb Cost" => Climb(ClimbSkill::Cost),
        "Climb Speed" => Climb(ClimbSkill::Speed),
        "Swim Speed" => Swim(SwimSkill::Speed),
        "Pick Speed" => Pick(MiningSkill::Speed),
        "Pick GemGain" => Pick(MiningSkill::GemGain),
        "Pick OreGain" => Pick(MiningSkill::OreGain),
        "Unlock Weapon Sword" => UnlockGroup(SkillGroupKind::Weapon(ToolKind::Sword)),
        "Unlock Weapon Axe" => UnlockGroup(SkillGroupKind::Weapon(ToolKind::Axe)),
        "Unlock Weapon Hammer" => UnlockGroup(SkillGroupKind::Weapon(ToolKind::Hammer)),
        "Unlock Weapon Bow" => UnlockGroup(SkillGroupKind::Weapon(ToolKind::Bow)),
        "Unlock Weapon Staff" => UnlockGroup(SkillGroupKind::Weapon(ToolKind::Staff)),
        "Unlock Weapon Sceptre" => UnlockGroup(SkillGroupKind::Weapon(ToolKind::Sceptre)),
        _ => {
            panic!(
                "Tried to convert an unsupported string from the database: {}",
                skill_string
            )
        },
    }
}

pub fn skill_group_to_db_string(skill_group: comp::skills::SkillGroupKind) -> String {
    use comp::{item::tool::ToolKind, skills::SkillGroupKind::*};
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

pub fn db_string_to_skill_group(skill_group_string: &str) -> comp::skills::SkillGroupKind {
    use comp::{item::tool::ToolKind, skills::SkillGroupKind::*};
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
