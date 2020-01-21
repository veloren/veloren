use crate::comp::{
    Body, CharacterState, ControllerInputs, EcsStateData, ItemKind::Tool, PhysicsState,
    StateUpdate, Stats,
};
use vek::vec::{Vec2, Vec3};

/*
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
}*/

pub fn handle_move_dir(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    let (accel, speed): (f32, f32) = if ecs_data.physics.on_ground {
        let accel = 50.0;
        let speed = 120.0;
        (accel, speed)
    } else {
        let accel = 10.0;
        let speed = 100.0;
        (accel, speed)
    };

    // Move player according to move_dir
    update.vel.0 += Vec2::broadcast(ecs_data.dt.0)
        * ecs_data.inputs.move_dir
        * if update.vel.0.magnitude_squared() < speed.powf(2.0) {
            accel
        } else {
            0.0
        };

    // Set direction based on move direction
    let ori_dir = if update.character.is_attack() || update.character.is_block() {
        Vec2::from(ecs_data.inputs.look_dir).normalized()
    } else {
        Vec2::from(update.vel.0)
    };

    // Smooth orientation
    if ori_dir.magnitude_squared() > 0.0001
        && (update.ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
            > 0.001
    {
        update.ori.0 = vek::ops::Slerp::slerp(update.ori.0, ori_dir.into(), 9.0 * ecs_data.dt.0);
    }
}

pub fn handle_wield(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if ecs_data.inputs.primary.is_pressed() || ecs_data.inputs.secondary.is_pressed() {
        if let Some(Tool(_)) = ecs_data.stats.equipment.main.as_ref().map(|i| &i.kind) {
            update.character = CharacterState::Wielding(None);
        }
    }
}

pub fn handle_sit(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if ecs_data.inputs.sit.is_pressed() && ecs_data.physics.on_ground && ecs_data.body.is_humanoid()
    {
        update.character = CharacterState::Sit(None);
    }
}

pub fn handle_climb(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if (ecs_data.inputs.climb.is_pressed() || ecs_data.inputs.climb_down.is_pressed())
        && ecs_data.physics.on_wall.is_some()
        && ecs_data.body.is_humanoid()
    {
        update.character = CharacterState::Climb(None);
    }
}

pub fn handle_roll(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if ecs_data.inputs.roll.is_pressed()
        && ecs_data.physics.on_ground
        && ecs_data.body.is_humanoid()
    {
        update.character = CharacterState::Roll(None);
    }
}

pub fn handle_unwield(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if let CharacterState::Wielded(_) = update.character {
        if ecs_data.inputs.toggle_wield.is_pressed() {
            update.character = CharacterState::Idle(None);
        }
    }
}

pub fn handle_glide(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if ecs_data.inputs.glide.is_pressed()
        && !ecs_data.physics.on_ground
        && ecs_data.body.is_humanoid()
    {
        dbg!();
        update.character = CharacterState::Glide(None);
    }
}
