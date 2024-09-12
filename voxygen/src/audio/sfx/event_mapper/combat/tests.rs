use super::*;
use crate::audio::sfx::SfxEvent;
use common::{
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
                hit_timing: 0.0,
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
                    attack_effect: None,
                    multi_target: None,
                    simultaneous_hits: 1,
                    custom_combo: None,
                    dodgeable: common::comp::ability::Dodgeable::Roll,
                    precision_flank_multipliers: Default::default(),
                    precision_flank_invert: false,
                },
                ori_modifier: 1.0,
                ability_info: empty_ability_info(),
                frontend_specifier: None,
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
