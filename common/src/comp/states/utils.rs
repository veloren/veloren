use crate::comp::{
    ActionState, ActionState::*, AttackKind::*, BlockKind::*, Body, ControllerInputs,
    ItemKind::Tool, MoveState, MoveState::*, PhysicsState, Stats,
};

/// _Determines what ability a player has selected for their primary ability,
/// and returns the corresponding `ActionState` or Idle if nothing_
pub fn determine_primary_ability(stats: &Stats) -> ActionState {
    if let Some(Tool(_)) = stats.equipment.main.as_ref().map(|i| &i.kind) {
        Attack(BasicAttack(None))
    } else {
        Idle(None)
    }
}

/// _Determines what ability a player has selected for their primary ability,
/// and returns the corresponding `ActionState` or Idle if nothing_
pub fn determine_secondary_ability(stats: &Stats) -> ActionState {
    if let Some(Tool(_)) = stats.equipment.main.as_ref().map(|i| &i.kind) {
        Block(BasicBlock(None))
    } else {
        Idle(None)
    }
}

/// _Returns a `MoveState` based on `in_fluid` condition_
pub fn determine_fall_or_swim(physics: &PhysicsState) -> MoveState {
    // Check if in fluid to go to swimming or back to falling
    if physics.in_fluid {
        Swim(None)
    } else {
        Fall(None)
    }
}
/// _Returns a `MoveState` based on `move_dir` magnitude_
pub fn determine_stand_or_run(inputs: &ControllerInputs) -> MoveState {
    // Return to running or standing based on move inputs
    if inputs.move_dir.magnitude_squared() > 0.0 {
        Run(None)
    } else {
        Stand(None)
    }
}

/// _Returns a `MoveState` based on `on_ground` state._
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

/// _Returns an ActionState based on whether character has a weapon equipped._
pub fn attempt_wield(stats: &Stats) -> ActionState {
    if let Some(Tool(_)) = stats.equipment.main.as_ref().map(|i| &i.kind) {
        Wield(None)
    } else {
        Idle(None)
    }
}

pub fn can_climb(physics: &PhysicsState, inputs: &ControllerInputs, body: &Body) -> bool {
    if let (true, Some(_wall_dir)) = (
        (inputs.climb.is_pressed() | inputs.climb_down.is_pressed()) && body.is_humanoid(),
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
