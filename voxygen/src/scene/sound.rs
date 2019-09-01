use crate::{
    audio::AudioFrontend,
};
use common::comp::{
    Pos,
    Body,
    CharacterState,
    MovementState::*,
};
use client::Client;
use vek::*;
use specs::{Entity as EcsEntity, Join};
use hashbrown::HashMap;
use std::{f32, time::Instant};

pub struct AnimState {
    last_step_sound: Instant,
}

pub struct SoundMgr {
    character_states: HashMap<EcsEntity, AnimState>
}

impl SoundMgr {
    pub fn new() -> Self {
        Self {
           character_states: HashMap::new(),
        }
    }

    pub fn maintain(&mut self, audio: &mut AudioFrontend, client: &Client) {
        let time = client.state().get_time();
        let tick = client.get_tick();
        let ecs = client.state().ecs();
        let dt = client.state().get_delta_time();
        // Get player position.
        let player_pos = ecs
            .read_storage::<Pos>()
            .get(client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        for (entity, pos, body, character) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<CharacterState>().maybe(),
        )
            .join()
        {
            if let Body::Humanoid(_) = body {
                let character = match character {
                    Some(c) => c,
                    _ => continue,
                };
                let state = self
                    .character_states
                    .entry(entity)
                    .or_insert_with(|| AnimState {last_step_sound: Instant::now()});

                if let Run = &character.movement {
                    if state.last_step_sound.elapsed().as_secs_f64() > 0.5 {
                        let rand_step = (rand::random::<usize>() % 7) + 1;
                        audio.play_sound(format!("voxygen.audio.footsteps.stepdirt_{}", rand_step));
                        state.last_step_sound = Instant::now();
                    }
                }
            }
        }
    }
}
