use crate::{
    comp::{AbilityState, CharacterState, EnergySource, ItemKind::Tool, StateUpdate},
    event::LocalEvent,
    sys::{character_behavior::JoinData, phys::GRAVITY},
};
use std::time::Duration;
use vek::vec::{Vec2, Vec3};

const HUMANOID_WATER_ACCEL: f32 = 70.0;
const HUMANOID_WATER_SPEED: f32 = 120.0;

pub fn handle_move(data: &JoinData, update: &mut StateUpdate) {
    if data.physics.in_fluid {
        swim_move(data, update);
    } else {
        ground_move(data, update);
    }
}

fn ground_move(data: &JoinData, update: &mut StateUpdate) {
    let (accel, speed): (f32, f32) = if data.physics.on_ground {
        let accel = 100.0;
        let speed = 9.0;
        (accel, speed)
    } else {
        let accel = 100.0;
        let speed = 8.0;
        (accel, speed)
    };

    // Move player according to move_dir
    if update.vel.0.magnitude_squared() < speed.powf(2.0) {
        update.vel.0 = update.vel.0 + Vec2::broadcast(data.dt.0) * data.inputs.move_dir * accel;
        let mag2 = update.vel.0.magnitude_squared();
        if mag2 > speed.powf(2.0) {
            update.vel.0 = update.vel.0.normalized() * speed;
        }
    }

    // Set direction based on move direction
    let ori_dir = if update.character.is_wield()
        || update.character.is_attack()
        || update.character.is_block()
    {
        Vec2::from(data.inputs.look_dir).normalized()
    } else {
        Vec2::from(update.vel.0)
    };

    // Smooth orientation
    if ori_dir.magnitude_squared() > 0.0001
        && (update.ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
            > 0.001
    {
        update.ori.0 = vek::ops::Slerp::slerp(update.ori.0, ori_dir.into(), 9.0 * data.dt.0);
    }
}

fn swim_move(data: &JoinData, update: &mut StateUpdate) {
    // Update velocity
    update.vel.0 += Vec2::broadcast(data.dt.0)
        * data.inputs.move_dir
        * if update.vel.0.magnitude_squared() < HUMANOID_WATER_SPEED.powf(2.0) {
            HUMANOID_WATER_ACCEL
        } else {
            0.0
        };

    // Set direction based on move direction when on the ground
    let ori_dir = if update.character.is_attack() || update.character.is_block() {
        Vec2::from(data.inputs.look_dir).normalized()
    } else {
        Vec2::from(update.vel.0)
    };

    if ori_dir.magnitude_squared() > 0.0001
        && (update.ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
            > 0.001
    {
        update.ori.0 = vek::ops::Slerp::slerp(
            update.ori.0,
            ori_dir.into(),
            if data.physics.on_ground { 9.0 } else { 2.0 } * data.dt.0,
        );
    }

    // Force players to pulse jump button to swim up
    if data.inputs.jump.is_pressed() && !data.inputs.jump.is_long_press(Duration::from_millis(600))
    {
        update.vel.0.z = (update.vel.0.z + data.dt.0 * GRAVITY * 1.25).min(HUMANOID_WATER_SPEED);
    }
}

pub fn handle_wield(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.primary.is_pressed() {
        if let Some(Tool(tool)) = data.stats.equipment.main.as_ref().map(|i| i.kind) {
            update.character = CharacterState::Equipping {
                tool,
                time_left: tool.equip_time(),
            };
        } else {
            update.character = CharacterState::Idle {};
        };
    }
}

pub fn handle_sit(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.sit.is_pressed() && data.physics.on_ground && data.body.is_humanoid() {
        update.character = CharacterState::Sit {};
    }
}

pub fn handle_climb(data: &JoinData, update: &mut StateUpdate) {
    if (data.inputs.climb.is_pressed() || data.inputs.climb_down.is_pressed())
        && data.physics.on_wall.is_some()
        && !data.physics.on_ground
        //&& update.vel.0.z < 0.0
        && data.body.is_humanoid()
    {
        update.character = CharacterState::Climb {};
    }
}

pub fn handle_unwield(data: &JoinData, update: &mut StateUpdate) {
    if let CharacterState::Wielding { .. } = update.character {
        if data.inputs.toggle_wield.is_pressed() {
            update.character = CharacterState::Idle {};
        }
    }
}

pub fn handle_glide(data: &JoinData, update: &mut StateUpdate) {
    if let CharacterState::Idle { .. } | CharacterState::Wielding { .. } = update.character {
        if data.inputs.glide.is_pressed() && !data.physics.on_ground && data.body.is_humanoid() {
            update.character = CharacterState::Glide {};
        }
    }
}

pub fn handle_jump(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.jump.is_pressed() && data.physics.on_ground {
        update
            .local_events
            .push_front(LocalEvent::Jump(data.entity));
    }
}

pub fn handle_primary(data: &JoinData, update: &mut StateUpdate) {
    if let Some(state) = data.ability_pool.primary {
        if let CharacterState::Wielding { .. } = update.character {
            if data.inputs.primary.is_pressed() {
                // data.updater.insert(data.entity, state);
                update.character = character_state_from_ability(data, state);
            }
        }
    }
}

pub fn handle_secondary(data: &JoinData, update: &mut StateUpdate) {
    if let Some(state) = data.ability_pool.secondary {
        if let CharacterState::Wielding { .. } = update.character {
            if data.inputs.secondary.is_pressed() {
                // data.updater.insert(data.entity, state);
                update.character = character_state_from_ability(data, state);
            }
        }
    }
}

pub fn handle_dodge(data: &JoinData, update: &mut StateUpdate) {
    if let Some(state) = data.ability_pool.dodge {
        if let CharacterState::Idle { .. } | CharacterState::Wielding { .. } = update.character {
            if data.inputs.roll.is_pressed()
                && data.physics.on_ground
                && data.body.is_humanoid()
                && update
                    .energy
                    .try_change_by(-200, EnergySource::Roll)
                    .is_ok()
            {
                // let tool_data =
                //     if let Some(Tool(data)) = data.stats.equipment.main.as_ref().map(|i|
                // i.kind) {         data
                //     } else {
                //         ToolData::default()
                //     };
                update.character = CharacterState::Roll {
                    remaining_duration: Duration::from_millis(600), // tool_data.attack_duration(),
                };
                data.updater.insert(data.entity, state);
            }
        }
    }
}

pub fn character_state_from_ability(
    data: &JoinData,
    ability_state: AbilityState,
) -> CharacterState {
    match ability_state {
        AbilityState::BasicAttack { .. } => {
            if let Some(Tool(tool)) = data.stats.equipment.main.as_ref().map(|i| i.kind) {
                CharacterState::BasicAttack {
                    exhausted: false,
                    remaining_duration: tool.attack_duration(),
                }
            } else {
                CharacterState::Idle {}
            }
        },
        AbilityState::BasicBlock { .. } => CharacterState::BasicBlock {},
        AbilityState::Roll { .. } => CharacterState::Roll {
            remaining_duration: Duration::from_millis(600),
        },
    }
}
