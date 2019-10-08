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

                if Self::trigger_should_play(last_play_entry, sfx_trigger_item) {
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

    /// When specific entity movements are detected, the associated sound (if any) needs to satisfy two conditions to
    /// be allowed to play:
    /// 1. An sfx config and files have been configured for the movement (we need to know which sound file(s) to play)
    /// 2. The sfx has not been played since it's timeout threshold has elapsed (we don't want to fire it repeatedly if the movement is long)
    fn trigger_should_play(
        last_play_entry: Option<&Instant>,
        sfx_trigger_item: Option<&SfxTriggerItem>,
    ) -> bool {
        match (last_play_entry, sfx_trigger_item) {
            (Some(last_play_entry), Some(sfx_trigger_item)) => {
                last_play_entry.elapsed().as_secs_f64() > sfx_trigger_item.threshold
            }
            (None, Some(_)) => true,
            _ => false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use common::comp::MovementState;
    use std::time::Instant;

    #[test]
    fn no_item_config() {
        let result = SoundMgr::trigger_should_play(None, None);

        assert_eq!(result, false);
    }

    #[test]
    fn config_but_played_since_threshold() {
        let trigger_item = SfxTriggerItem {
            trigger: MovementState::Run,
            files: vec![String::from("some.path.to.sfx.file")],
            threshold: 1.0,
        };

        let result = SoundMgr::trigger_should_play(Some(&Instant::now()), Some(&trigger_item));

        assert_eq!(result, false);
    }

    #[test]
    fn config_and_never_played() {
        let trigger_item = SfxTriggerItem {
            trigger: MovementState::Run,
            files: vec![String::from("some.path.to.sfx.file")],
            threshold: 1.0,
        };

        let result = SoundMgr::trigger_should_play(None, Some(&trigger_item));

        assert_eq!(result, true);
    }

    #[test]
    fn config_and_not_played_since_threshold() {
        let trigger_item = SfxTriggerItem {
            trigger: MovementState::Run,
            files: vec![String::from("some.path.to.sfx.file")],
            threshold: 0.0,
        };

        let instant = Instant::now();
        let result = SoundMgr::trigger_should_play(Some(&instant), Some(&trigger_item));

        assert_eq!(result, true);
    }
}
