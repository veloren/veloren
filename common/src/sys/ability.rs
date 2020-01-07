#![allow(unused_imports)]
#![allow(dead_code)]
use crate::comp::{
    AbilityAction, AbilityActionKind, AbilityPool, ActionState::*, AttackKind, CharacterState,
    StateHandler,
};

use specs::{Entities, Join, LazyUpdate, Read, ReadStorage, System, WriteStorage};

/// # Ability System
/// #### Updates tuples of ( `CharacterState`, `AbilityAction`, and `AbilityPool`s )
/// _Each update determines what type of ability is being started, and looks into the AbilityPool for which
///  Ability that should be used. System then updates `CharacterState` to that Ability._
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, LazyUpdate>,
        WriteStorage<'a, CharacterState>,
        ReadStorage<'a, AbilityAction>,
        ReadStorage<'a, AbilityPool>,
    );
    fn run(
        &mut self,
        (
            entities,
            _updater,
            mut character_state_storage,
            ability_action_storage,
            ability_pool_storage,
        ): Self::SystemData,
    ) {
        for (_entity, mut _character, _ability_action, _ability_pool) in (
            &entities,
            &mut character_state_storage,
            &ability_action_storage,
            &ability_pool_storage,
        )
            .join()
        {
            // match ability_action.0 {
            //     AbilityActionKind::Primary => {
            //         if let Some(AttackKind(Some(attack_kind))) = ability_pool.primary {
            //             character.action_state = Attack(attack_kind::default());
            //         }
            //     }
            //     AbilityActionKind::Secondary => {
            //         if let Some(attack_kind) = ability_pool.secondary {
            //             character.action_state = Attack(attack_kind::default());
            //         }
            //     }
            //     AbilityActionKind::Block => {
            //         if let Some(block_kind) = ability_pool.block {
            //             character.action_state = Block(block_kind::default());
            //         }
            //     }
            //     AbilityActionKind::Dodge => {
            //         if let Some(dodge_kind) = ability_pool.dodge {
            //             character.action_state = Dodge(dodge_kind::default());
            //         }
            //     }
            //     _ => {}
            // }
        }
    }
}
