use crate::audio::AudioFrontend;
use client::Client;
use common::comp::{Body, CharacterState, MovementState, Ori, Pos};
use hashbrown::HashMap;
use ron::de::from_str;
use serde::Deserialize;
use specs::{Entity as EcsEntity, Join};
use std::time::Instant;
use vek::*;

#[derive(Deserialize)]
struct SfxTriggerItem {
    trigger: MovementState,
    files: Vec<String>,
    threshold: f64,
}

#[derive(Deserialize)]
struct SfxTriggers {
    items: Vec<SfxTriggerItem>,
}

pub struct SfxTriggerState {
    trigger_history: HashMap<MovementState, Instant>,
}

pub struct SoundMgr {
    character_states: HashMap<EcsEntity, SfxTriggerState>,
    triggers: SfxTriggers,
}

impl SoundMgr {
    pub fn new() -> Self {
        Self {
            character_states: HashMap::new(),
            triggers: Self::load_sfx_items(),
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

        for (entity, pos, body, character) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<CharacterState>().maybe(),
        )
            .join()
            .filter(|(_, e_pos, _, _)| (e_pos.0.distance_squared(player_pos)) < SFX_DIST_LIMIT_SQR)
        {
            if let (Body::Humanoid(_), Some(character)) = (body, character) {
                let state =
                    self.character_states
                        .entry(entity)
                        .or_insert_with(|| SfxTriggerState {
                            trigger_history: HashMap::new(),
                        });

                let last_play_entry = state.trigger_history.get(&character.movement);

                // Check for SFX config entry for this movement
                let sfx_trigger_item: Option<&SfxTriggerItem> = self
                    .triggers
                    .items
                    .iter()
                    .find(|item| item.trigger == character.movement);

                // Check valid sfx config and whether wait threshold has elapsed
                let can_play = match (last_play_entry, sfx_trigger_item) {
                    (Some(last_play_entry), Some(sfx_trigger_item)) => {
                        last_play_entry.elapsed().as_secs_f64() > sfx_trigger_item.threshold
                    }
                    (None, Some(_)) => true,
                    _ => false,
                };

                if can_play {
                    // Update the last play time
                    state
                        .trigger_history
                        .insert(character.movement, Instant::now());

                    let item = sfx_trigger_item.unwrap();

                    let sfx_file = match item.files.len() {
                        1 => item.files.last().unwrap(),
                        _ => {
                            let rand_step = rand::random::<usize>() % item.files.len();
                            &item.files[rand_step]
                        }
                    };

                    audio.play_sound(sfx_file, pos.0);
                }
            }
        }
    }

    fn load_sfx_items() -> SfxTriggers {
        // slapping it here while the format is in flux
        const CONFIG: &str = "
    (
      items: [
        (
            trigger: Run,
            files: [
                \"voxygen.audio.sfx.footsteps.stepdirt_1\",
                \"voxygen.audio.sfx.footsteps.stepdirt_2\",
                \"voxygen.audio.sfx.footsteps.stepdirt_3\",
                \"voxygen.audio.sfx.footsteps.stepdirt_4\",
                \"voxygen.audio.sfx.footsteps.stepdirt_5\",
            ],
            threshold: 0.25,
        ),
      ],
    )";

        let collection: SfxTriggers = match from_str(CONFIG) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {}", e);

                std::process::exit(1);
            }
        };

        collection
    }
}
