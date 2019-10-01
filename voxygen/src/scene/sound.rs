use crate::audio::fader::Fader;
use crate::audio::AudioFrontend;
use client::Client;
use common::comp::{Body, CharacterState, MovementState, MovementState::*, Ori, Pos, Vel};
use hashbrown::HashMap;
use specs::{Entity as EcsEntity, Join};
use std::time::Instant;
use vek::*;

// TODO this is going to get very large...
pub struct AnimState {
    last_character_movement: MovementState,
    last_step_sound: Instant,
    last_jump_sound: Instant,
    last_attack_sound: Instant,
    last_environment_sound: Instant,
    last_glider_open_sound: Instant,
    last_glide_sound: Instant,
    gliding_channel: usize,
}

pub struct SoundMgr {
    character_states: HashMap<EcsEntity, AnimState>,
}

impl SoundMgr {
    pub fn new() -> Self {
        Self {
            character_states: HashMap::new(),
        }
    }

    pub fn maintain(&mut self, audio: &mut AudioFrontend, client: &Client) {
        const SFX_DIST_LIMIT_SQR: f32 = 22500.0;
        let ecs = client.state().ecs();

        // Get player position.
        let player_pos = ecs
            .read_storage::<Pos>()
            .get(client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        let player_ori = ecs
            .read_storage::<Ori>()
            .get(client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        audio.set_listener_pos(&player_pos, &player_ori);

        for (entity, pos, body, vel, character) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Body>(),
            &ecs.read_storage::<Vel>(),
            ecs.read_storage::<CharacterState>().maybe(),
        )
            .join()
            .filter(|(_, e_pos, _, _)| (e_pos.0.distance_squared(player_pos)) < SFX_DIST_LIMIT_SQR)
        {
            if let (Body::Humanoid(_), Some(character), vel) = (body, character, vel) {
                let state = self
                    .character_states
                    .entry(entity)
                    .or_insert_with(|| AnimState {
                        last_character_movement: Stand,
                        last_step_sound: Instant::now(),
                        last_jump_sound: Instant::now(),
                        last_attack_sound: Instant::now(),
                        last_environment_sound: Instant::now(),
                        last_glider_open_sound: Instant::now(),
                        last_glide_sound: Instant::now(),
                        gliding_channel: 0,
                    });

                // Constrain to our player for testing
                if entity == client.entity() {
                    // Ambient sounds
                    if state.last_environment_sound.elapsed().as_secs_f64() > 60.0 {
                        let chunk = client.current_chunk();

                        if let Some(chunk) = chunk {
                            let biome = chunk.meta().biome();
                            let time_of_day = client.state().get_time_of_day();

                            log::warn!("{}", format!("Biome: {:?}", biome));
                            log::warn!("{}", format!("Time of Day: {:#?}", time_of_day));

                            state.last_environment_sound = Instant::now();
                        }
                    }

                    // Attack
                    if character.action.is_attack()
                        && state.last_attack_sound.elapsed().as_secs_f64() > 0.25
                    {
                        let rand_item = (rand::random::<usize>() % 2) + 1;
                        audio.play_sound(
                            &format!("voxygen.audio.sfx.attack.attack_{}", rand_item),
                            pos.0,
                        );
                        state.last_attack_sound = Instant::now();
                        state.last_character_movement = MovementState::Stand;
                    }

                    // Glider Open
                    if character.movement == MovementState::Glide
                        && state.last_glider_open_sound.elapsed().as_secs_f64() > 1.0
                        && state.last_character_movement != MovementState::Glide
                    {
                        state.gliding_channel =
                            audio.play_sound("voxygen.audio.sfx.glider.open", pos.0);
                        state.last_character_movement = Glide;
                    }

                    // Gliding
                    if character.movement == MovementState::Glide
                        && state.last_glide_sound.elapsed().as_secs_f64() > 2.5
                        && state.last_character_movement == MovementState::Glide
                    {
                        state.gliding_channel =
                            audio.play_sound("voxygen.audio.sfx.glider.gliding", pos.0);

                        state.last_character_movement = Glide;
                        state.last_glide_sound = Instant::now();
                    }

                    // Glider Close
                    if state.last_character_movement == MovementState::Glide
                        && character.movement != MovementState::Glide
                        && state.last_glider_open_sound.elapsed().as_secs_f64() > 1.0
                    {
                        audio.play_sound("voxygen.audio.sfx.glider.open", pos.0);
                        audio.stop_channel(state.gliding_channel, Fader::fade_out(0.5));

                        state.last_character_movement = Stand;
                    }

                    // Jump
                    if character.movement == MovementState::Jump
                        && vel.0.z > 0.0
                        && state.last_jump_sound.elapsed().as_secs_f64() > 0.25
                    {
                        let rand_item = (rand::random::<usize>() % 2) + 1;
                        audio.play_sound(
                            &format!("voxygen.audio.sfx.jump.jump_{}", rand_item),
                            pos.0,
                        );
                        state.last_jump_sound = Instant::now();
                        state.last_character_movement = MovementState::Jump;
                    }
                }

                if character.movement == Run && state.last_step_sound.elapsed().as_secs_f64() > 0.25
                {
                    let rand_step = (rand::random::<usize>() % 5) + 1;
                    audio.play_sound(
                        &format!("voxygen.audio.sfx.steps.step_{}", rand_step),
                        pos.0,
                    );
                    state.last_step_sound = Instant::now();
                }
            }
        }
    }
}
