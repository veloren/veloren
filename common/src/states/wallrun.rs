use super::utils::*;
use crate::{
    comp::{CharacterState, InputKind, StateUpdate, character_state::OutputEvents},
    states::{
        behavior::{CharacterBehavior, JoinData},
        idle, wielding,
    },
};
use serde::{Deserialize, Serialize};
use vek::Vec2;

const WALLRUN_ANTIGRAV: f32 = crate::consts::GRAVITY * 0.5;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// Had weapon
    pub was_wielded: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        if self.was_wielded {
            attempt_input(data, output_events, &mut update);
        } else {
            handle_wield(data, &mut update);
        }

        handle_climb(data, &mut update);

        {
            let lift = WALLRUN_ANTIGRAV;
            update.vel.0.z += data.dt.0
                * lift
                * (Vec2::<f32>::from(update.vel.0).magnitude() * 0.075).clamp(0.2, 1.0);
        }

        // fall off wall, hit ground, or enter water
        // TODO: Rugged way to determine when state change occurs and we need to leave
        // this state
        if data.physics.on_wall.is_none()
            || data.physics.on_ground.is_some()
            || data.physics.in_liquid().is_some()
        {
            update.character = if self.was_wielded {
                CharacterState::Wielding(wielding::Data { is_sneaking: false })
            } else {
                CharacterState::Idle(idle::Data::default())
            };
        }

        update
    }

    fn on_input(
        &self,
        data: &JoinData,
        input: InputKind,
        output_events: &mut OutputEvents,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if matches!(input, InputKind::Jump) {
            handle_walljump(data, output_events, &mut update, self.was_wielded);
        }
        update
    }
}
