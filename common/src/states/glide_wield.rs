use super::utils::*;
use crate::{
    comp::{
        character_state::OutputEvents, controller::InputKind, slot::EquipSlot, CharacterState,
        InventoryAction, Ori, StateUpdate,
    },
    event::LocalEvent,
    outcome::Outcome,
    states::{
        behavior::{CharacterBehavior, JoinData},
        glide, idle,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub ori: Ori,
    span_length: f32,
    chord_length: f32,
}

impl From<&JoinData<'_>> for Data {
    fn from(data: &JoinData) -> Self {
        let scale = data.body.dimensions().z.sqrt();
        Self {
            // Aspect ratio is what really matters for lift/drag ratio
            // and the aerodynamics model works for ARs up to 25.
            //
            // The inflated dimensions are hopefully only a temporary
            // bandaid for the poor glide ratio experienced under 2.5G.
            //
            // The formula is:
            //  s: span_length_modifier
            //  c: chord_length_modifier
            //  h: height (this is a hack to balance different races)
            //
            // p_a = Pi/4 * c * h * s * h
            // AR
            //  = (s * h)^2 / p_a
            //  = (s * h)^2  / (Pi / 4 * (c * h) * (s * h))
            //  = (s * h) / (c * h) / (Pi / 4)
            //  = s / c / Pi/4
            //
            // or if c is 1,
            //  = s / Pi/4
            //
            // In other words, the bigger `span_length` the better.
            //
            // A span/chord ratio of 4.5 gives an AR of ~5.73.
            // A span/chord ratio of 3.0 gives an ARI of ~3.82.
            span_length: scale * 3.0,
            chord_length: scale,
            ori: *data.ori,
        }
    }
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 1.0);
        handle_jump(data, output_events, &mut update, 1.0);
        if input_is_pressed(data, InputKind::Roll) {
            handle_input(data, output_events, &mut update, InputKind::Roll);
        }
        handle_wield(data, &mut update);

        // If still in this state, do the things
        if matches!(update.character, CharacterState::GlideWield(_)) {
            // If not on the ground while wielding glider enter gliding state
            update.character = if data.physics.on_ground.is_none() {
                CharacterState::Glide(glide::Data::new(
                    self.span_length,
                    self.chord_length,
                    self.ori,
                ))
            // make sure we have a glider and we're not (too deep) in water
            } else if data
                .inventory
                .and_then(|inv| inv.equipped(EquipSlot::Glider))
                .is_some()
                && data.physics.in_liquid().map_or(true, |depth| depth < 0.5)
            {
                CharacterState::GlideWield(Self {
                    // Glider tilt follows look dir
                    ori: self.ori.slerped_towards(
                        data.ori.slerped_towards(
                            Ori::from(data.inputs.look_dir).pitched_up(0.6),
                            (1.0 + data.inputs.look_dir.dot(*data.ori.look_dir()).max(0.0)) / 3.0,
                        ),
                        5.0 * data.dt.0,
                    ),
                    ..*self
                })
            } else {
                CharacterState::Idle(idle::Data::default())
            };
        }

        update
    }

    fn manipulate_loadout(
        &self,
        data: &JoinData,
        output_events: &mut OutputEvents,
        inv_action: InventoryAction,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_manipulate_loadout(data, output_events, &mut update, inv_action);
        update
    }

    fn unwield(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        output_events.emit_local(LocalEvent::CreateOutcome(Outcome::Glider {
            pos: data.pos.0,
            wielded: false,
        }));
        update.character = CharacterState::Idle(idle::Data::default());
        update
    }

    fn sit(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sit(data, &mut update);
        update
    }

    fn dance(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_dance(data, &mut update);
        update
    }

    fn sneak(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sneak(data, &mut update);
        update
    }
}
