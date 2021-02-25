use super::*;
use crate::audio::sfx::SfxEvent;
use common::{
    comp::{
        inventory::loadout_builder::LoadoutBuilder, item::tool::ToolKind, CharacterAbilityType,
        CharacterState, Item,
    },
    states,
};
use std::time::{Duration, Instant};

#[test]
fn maps_wield_while_equipping() {
    let loadout = LoadoutBuilder::new()
        .active_item(Some(Item::new_from_asset_expect(
            "common.items.weapons.axe.starter_axe",
        )))
        .build();
    let inventory = Inventory::new_with_loadout(loadout);

    let result = CombatEventMapper::map_event(
        &CharacterState::Equipping(states::equipping::Data {
            static_data: states::equipping::StaticData {
                buildup_duration: Duration::from_millis(10),
            },
            timer: Duration::default(),
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
    let loadout = LoadoutBuilder::new()
        .active_item(Some(Item::new_from_asset_expect(
            "common.items.weapons.bow.starter",
        )))
        .build();
    let inventory = Inventory::new_with_loadout(loadout);

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
    let loadout = LoadoutBuilder::new()
        .active_item(Some(Item::new_from_asset_expect(
            "common.items.weapons.axe.starter_axe",
        )))
        .build();
    let inventory = Inventory::new_with_loadout(loadout);

    let result = CombatEventMapper::map_event(
        &CharacterState::BasicMelee(states::basic_melee::Data {
            static_data: states::basic_melee::StaticData {
                buildup_duration: Duration::default(),
                swing_duration: Duration::default(),
                recover_duration: Duration::default(),
                base_damage: 10.0,
                base_poise_damage: 10.0,
                knockback: 0.0,
                range: 1.0,
                max_angle: 1.0,
                ability_info: empty_ability_info(),
            },
            timer: Duration::default(),
            stage_section: states::utils::StageSection::Buildup,
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
        SfxEvent::Attack(CharacterAbilityType::BasicMelee, ToolKind::Axe)
    );
}

#[test]
fn matches_ability_stage() {
    let loadout = LoadoutBuilder::new()
        .active_item(Some(Item::new_from_asset_expect(
            "common.items.weapons.sword.starter",
        )))
        .build();
    let inventory = Inventory::new_with_loadout(loadout);

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
                    base_recover_duration: Duration::from_millis(400),
                    forward_movement: 0.5,
                }],
                initial_energy_gain: 0.0,
                max_energy_gain: 100.0,
                energy_increase: 20.0,
                speed_increase: 0.05,
                max_speed_increase: 0.8,
                scales_from_combo: 2,
                is_interruptible: true,
                ability_info: empty_ability_info(),
            },
            stage: 1,
            combo: 0,
            timer: Duration::default(),
            stage_section: states::utils::StageSection::Swing,
            next_stage: false,
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
            CharacterAbilityType::ComboMelee(states::utils::StageSection::Swing, 1),
            ToolKind::Sword
        )
    );
}

#[test]
fn ignores_different_ability_stage() {
    let loadout = LoadoutBuilder::new()
        .active_item(Some(Item::new_from_asset_expect(
            "common.items.weapons.axe.starter_axe",
        )))
        .build();
    let inventory = Inventory::new_with_loadout(loadout);

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
                    base_recover_duration: Duration::from_millis(400),
                    forward_movement: 0.5,
                }],
                initial_energy_gain: 0.0,
                max_energy_gain: 100.0,
                energy_increase: 20.0,
                speed_increase: 0.05,
                max_speed_increase: 0.8,
                scales_from_combo: 2,
                is_interruptible: true,
                ability_info: empty_ability_info(),
            },
            stage: 1,
            combo: 0,
            timer: Duration::default(),
            stage_section: states::utils::StageSection::Swing,
            next_stage: false,
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
            CharacterAbilityType::ComboMelee(states::utils::StageSection::Swing, 2),
            ToolKind::Sword
        )
    );
}

fn empty_ability_info() -> states::utils::AbilityInfo {
    states::utils::AbilityInfo {
        tool: None,
        hand: None,
        key: states::utils::AbilityKey::Mouse1,
    }
}
