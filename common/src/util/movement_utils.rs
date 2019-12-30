use crate::comp::TEMP_EQUIP_DELAY;
use crate::comp::{
    ActionState, ActionState::*, AttackKind::*, BasicAttackState, BasicBlockState, BlockKind::*,
    Body, ControllerInputs, FallState, IdleState, ItemKind::Tool, MoveState, MoveState::*,
    PhysicsState, RunState, StandState, Stats, SwimState, ToolData, WieldState,
};
use std::time::Duration;

/// _Determines what ability a player has selected for their primary ability,
/// and returns the corresponding `ActionState`
/// ... or Idle if nothing it possible?_
pub fn determine_primary_ability(stats: &Stats) -> ActionState {
    if let Some(Tool(data)) = stats.equipment.main.as_ref().map(|i| &i.kind) {
        Attack(BasicAttack(BasicAttackState {
            remaining_duration: data.attack_duration(),
        }))
    } else {
        Idle(IdleState)
    }
}

/// _Determines what ability a player has selected for their primary ability,
/// and returns the corresponding `ActionState`
/// ... or Idle if nothing it possible?_
pub fn determine_secondary_ability(stats: &Stats) -> ActionState {
    if let Some(Tool(data)) = stats.equipment.main.as_ref().map(|i| &i.kind) {
        Block(BasicBlock(BasicBlockState {
            active_duration: Duration::default(),
        }))
    } else {
        Idle(IdleState)
    }
}

/// __Returns a `MoveState` based on `in_fluid` condition__
pub fn determine_fall_or_swim(physics: &PhysicsState) -> MoveState {
    // Check if in fluid to go to swimming or back to falling
    if physics.in_fluid {
        Swim(SwimState)
    } else {
        Fall(FallState)
    }
}
/// __Returns a `MoveState` based on `move_dir` magnitude__
pub fn determine_stand_or_run(inputs: &ControllerInputs) -> MoveState {
    // Return to running or standing based on move inputs
    if inputs.move_dir.magnitude_squared() > 0.0 {
        Run(RunState)
    } else {
        Stand(StandState)
    }
}

/// __Returns a `MoveState` based on `on_ground` state.__
///
/// _`FallState`, or `SwimState` if not `on_ground`,
/// `StandState` or `RunState` if is `on_ground`_
pub fn determine_move_from_grounded_state(
    physics: &PhysicsState,
    inputs: &ControllerInputs,
) -> MoveState {
    // Not on ground, go to swim or fall
    if !physics.on_ground {
        determine_fall_or_swim(physics)
    }
    // On ground
    else {
        determine_stand_or_run(inputs)
    }
}

/// __Returns an ActionState based on whether character has a weapon equipped.__
pub fn attempt_wield(stats: &Stats) -> ActionState {
    if let Some(Tool { .. }) = stats.equipment.main.as_ref().map(|i| &i.kind) {
        Wield(WieldState {
            equip_delay: Duration::from_millis(TEMP_EQUIP_DELAY),
        })
    } else {
        Idle(IdleState)
    }
}

pub fn can_climb(physics: &PhysicsState, inputs: &ControllerInputs, body: &Body) -> bool {
    if let (true, Some(_wall_dir)) = (
        inputs.climb.is_pressed() | inputs.climb_down.is_pressed() && body.is_humanoid(),
        physics.on_wall,
    ) {
        true
    } else {
        false
    }
}

pub fn can_glide(physics: &PhysicsState, inputs: &ControllerInputs, body: &Body) -> bool {
    if inputs.glide.is_pressed() && body.is_humanoid() && physics.on_wall == None {
        true
    } else {
        false
    }
}

pub fn can_sit(physics: &PhysicsState, inputs: &ControllerInputs, body: &Body) -> bool {
    if inputs.sit.is_pressed() && physics.on_ground && body.is_humanoid() {
        true
    } else {
        false
    }
}

pub fn can_jump(physics: &PhysicsState, inputs: &ControllerInputs) -> bool {
    if physics.on_ground && inputs.jump.is_pressed() {
        true
    } else {
        false
    }
}
