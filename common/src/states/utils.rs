use crate::{
    comp::{Attacking, CharacterState, EcsStateData, EnergySource, ItemKind::Tool, StateUpdate},
    event::LocalEvent,
};
use std::time::Duration;
use vek::vec::{Vec2, Vec3};

pub fn handle_move_dir(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    let (accel, speed): (f32, f32) = if ecs_data.physics.on_ground {
        let accel = 100.0;
        let speed = 8.0;
        (accel, speed)
    } else {
        let accel = 100.0;
        let speed = 8.0;
        (accel, speed)
    };

    // Move player according to move_dir
    if update.vel.0.magnitude_squared() < speed.powf(2.0) {
        update.vel.0 =
            update.vel.0 + Vec2::broadcast(ecs_data.dt.0) * ecs_data.inputs.move_dir * accel;
        let mag2 = update.vel.0.magnitude_squared();
        if mag2 > speed.powf(2.0) {
            update.vel.0 = update.vel.0.normalized() * speed;
        }
    }

    // Set direction based on move direction
    let ori_dir = if update.character.is_wielded()
        || update.character.is_attack()
        || update.character.is_block()
    {
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
        && !ecs_data.physics.on_ground
        //&& update.vel.0.z < 0.0
        && ecs_data.body.is_humanoid()
    {
        update.character = CharacterState::Climb(None);
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
    if let CharacterState::Idle(_) | CharacterState::Wielded(_) = update.character {
        if ecs_data.inputs.glide.is_pressed()
            && !ecs_data.physics.on_ground
            && ecs_data.body.is_humanoid()
        {
            update.character = CharacterState::Glide(None);
        }
    }
}

pub fn handle_jump(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if ecs_data.inputs.jump.is_pressed() && ecs_data.physics.on_ground {
        update
            .local_events
            .push_front(LocalEvent::Jump(*ecs_data.entity));
    }
}

pub fn handle_primary(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if let Some(state) = ecs_data.ability_pool.primary {
        if let CharacterState::Wielded(_) = update.character {
            if ecs_data.inputs.primary.is_pressed() {
                update.character = state;
            }
        }
    }
}

pub fn handle_secondary(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if let Some(state) = ecs_data.ability_pool.secondary {
        if let CharacterState::Wielded(_) = update.character {
            if ecs_data.inputs.secondary.is_pressed() {
                update.character = state;
            }
        }
    }
}

pub fn handle_dodge(ecs_data: &EcsStateData, update: &mut StateUpdate) {
    if let Some(state) = ecs_data.ability_pool.dodge {
        if let CharacterState::Idle(_) | CharacterState::Wielded(_) = update.character {
            if ecs_data.inputs.roll.is_pressed()
                && ecs_data.physics.on_ground
                && ecs_data.body.is_humanoid()
                && update
                    .energy
                    .try_change_by(-200, EnergySource::Roll)
                    .is_ok()
            {
                update.character = state;
            }
        }
    }
}
