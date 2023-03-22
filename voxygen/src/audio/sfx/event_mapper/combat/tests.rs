use super::*;
use crate::audio::sfx::SfxEvent;
use common::{
    combat::DamageKind,
    comp::{
        controller::InputKind, inventory::loadout_builder::LoadoutBuilder, item::tool::ToolKind,
        melee, CharacterAbilityType, CharacterState, Item,
    },
    states,
};
use std::time::{Duration, Instant};

#[test]
fn maps_wield_while_equipping() {
    let loadout = LoadoutBuilder::empty()
        .active_mainhand(Some(Item::new_from_asset_expect(
            "common.items.weapons.axe.starter_axe",
        )))
        .build();
    let inventory = Inventory::with_loadout_humanoid(loadout);

    let result = CombatEventMapper::map_event(
        &CharacterState::Equipping(states::equipping::Data {
            static_data: states::equipping::StaticData {
                buildup_duration: Duration::from_millis(10),
            },
            timer: Duration::default(),
            is_sneaking: false,
        }),
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: false,
        },
        &inventory,
    );

    assert_eq!(result, SfxEvent::Wield(ToolKind::Axe));
}

#[test]
fn maps_unwield() {
    let loadout = LoadoutBuilder::empty()
        .active_mainhand(Some(Item::new_from_asset_expect(
            "common.items.weapons.bow.starter",
        )))
        .build();
    let inventory = Inventory::with_loadout_humanoid(loadout);

    let result = CombatEventMapper::map_event(
        &CharacterState::default(),
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: true,
        },
        &inventory,
    );

    assert_eq!(result, SfxEvent::Unwield(ToolKind::Bow));
}

#[test]
fn maps_basic_melee() {
    let loadout = LoadoutBuilder::empty()
        .active_mainhand(Some(Item::new_from_asset_expect(
            "common.items.weapons.axe.starter_axe",
        )))
        .build();
    let inventory = Inventory::with_loadout_humanoid(loadout);

    let result = CombatEventMapper::map_event(
        &CharacterState::BasicMelee(states::basic_melee::Data {
            static_data: states::basic_melee::StaticData {
                buildup_duration: Duration::default(),
                swing_duration: Duration::default(),
                recover_duration: Duration::default(),
                melee_constructor: melee::MeleeConstructor {
                    kind: melee::MeleeConstructorKind::Slash {
                        damage: 1.0,
                        knockback: 0.0,
                        poise: 0.0,
                        energy_regen: 0.0,
                    },
                    scaled: None,
                    range: 3.5,
                    angle: 15.0,
                    damage_effect: None,
                    multi_target: None,
                },
                ori_modifier: 1.0,
                ability_info: empty_ability_info(),
            },
            timer: Duration::default(),
            stage_section: states::utils::StageSection::Action,
            exhausted: false,
        }),
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: true,
        },
        &inventory,
    );

    assert_eq!(
        result,
        SfxEvent::Attack(
            CharacterAbilityType::BasicMelee(states::utils::StageSection::Action),
            ToolKind::Axe
        )
    );
}

#[test]
fn matches_ability_stage() {
    let loadout = LoadoutBuilder::empty()
        .active_mainhand(Some(Item::new_from_asset_expect(
            "common.items.weapons.sword.starter",
        )))
        .build();
    let inventory = Inventory::with_loadout_humanoid(loadout);

    let result = CombatEventMapper::map_event(
        &CharacterState::ComboMelee(states::combo_melee::Data {
            static_data: states::combo_melee::StaticData {
                num_stages: 1,
                stage_data: vec![states::combo_melee::Stage {
                    stage: 1,
                    base_damage: 100.0,
                    base_poise_damage: 100.0,
                    damage_increase: 10.0,
                    poise_damage_increase: 10.0,
                    knockback: 10.0,
                    range: 4.0,
                    angle: 30.0,
                    base_buildup_duration: Duration::from_millis(500),
                    base_swing_duration: Duration::from_millis(200),
                    hit_timing: 0.5,
                    base_recover_duration: Duration::from_millis(400),
                    forward_movement: 0.5,
                    damage_kind: DamageKind::Slashing,
                    damage_effect: None,
                }],
                initial_energy_gain: 0.0,
                max_energy_gain: 100.0,
                energy_increase: 20.0,
                speed_increase: 0.05,
                max_speed_increase: 0.8,
                scales_from_combo: 2,
                ori_modifier: 1.0,
                ability_info: empty_ability_info(),
            },
            exhausted: false,
            stage: 1,
            timer: Duration::default(),
            stage_section: states::utils::StageSection::Action,
        }),
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: true,
        },
        &inventory,
    );

    assert_eq!(
        result,
        SfxEvent::Attack(
            CharacterAbilityType::ComboMelee(states::utils::StageSection::Action, 1),
            ToolKind::Sword
        )
    );
}

#[test]
fn ignores_different_ability_stage() {
    let loadout = LoadoutBuilder::empty()
        .active_mainhand(Some(Item::new_from_asset_expect(
            "common.items.weapons.axe.starter_axe",
        )))
        .build();
    let inventory = Inventory::with_loadout_humanoid(loadout);

    let result = CombatEventMapper::map_event(
        &CharacterState::ComboMelee(states::combo_melee::Data {
            static_data: states::combo_melee::StaticData {
                num_stages: 1,
                stage_data: vec![states::combo_melee::Stage {
                    stage: 1,
                    base_damage: 100.0,
                    base_poise_damage: 100.0,
                    damage_increase: 100.0,
                    poise_damage_increase: 10.0,
                    knockback: 10.0,
                    range: 4.0,
                    angle: 30.0,
                    base_buildup_duration: Duration::from_millis(500),
                    base_swing_duration: Duration::from_millis(200),
                    hit_timing: 0.5,
                    base_recover_duration: Duration::from_millis(400),
                    forward_movement: 0.5,
                    damage_kind: DamageKind::Slashing,
                    damage_effect: None,
                }],
                initial_energy_gain: 0.0,
                max_energy_gain: 100.0,
                energy_increase: 20.0,
                speed_increase: 0.05,
                max_speed_increase: 0.8,
                scales_from_combo: 2,
                ori_modifier: 1.0,
                ability_info: empty_ability_info(),
            },
            exhausted: false,
            stage: 1,
            timer: Duration::default(),
            stage_section: states::utils::StageSection::Action,
        }),
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: true,
        },
        &inventory,
    );

    assert_ne!(
        result,
        SfxEvent::Attack(
            CharacterAbilityType::ComboMelee(states::utils::StageSection::Action, 2),
            ToolKind::Sword
        )
    );
}

fn empty_ability_info() -> states::utils::AbilityInfo {
    states::utils::AbilityInfo {
        tool: None,
        hand: None,
        input: InputKind::Primary,
        input_attr: None,
        ability_meta: Default::default(),
        ability: None,
    }
}
