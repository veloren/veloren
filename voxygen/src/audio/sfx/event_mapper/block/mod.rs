/// EventMapper::Block watches the sound emitting blocks within
/// chunk range of the player and emits ambient sfx
use crate::{
    audio::sfx::{SfxEvent, SfxEventItem, SfxTriggerItem, SfxTriggers, SFX_DIST_LIMIT_SQR},
    scene::{terrain::BlocksOfInterest, Camera, Terrain},
};

use super::EventMapper;
use client::Client;
use common::{
    comp::Pos, event::EventBus, spiral::Spiral2d, state::State, terrain::TerrainChunk,
    vol::RectRasterableVol,
};
use hashbrown::HashMap;
use rand::{thread_rng, Rng};
use specs::WorldExt;
use std::time::Instant;
use vek::*;

#[derive(Clone, PartialEq)]
struct PreviousBlockState {
    event: SfxEvent,
    time: Instant,
}

impl Default for PreviousBlockState {
    fn default() -> Self {
        Self {
            event: SfxEvent::Idle,
            time: Instant::now(),
        }
    }
}

pub struct BlockEventMapper {
    history: HashMap<Vec3<i32>, PreviousBlockState>,
}

impl EventMapper for BlockEventMapper {
    fn maintain(
        &mut self,
        state: &State,
        player_entity: specs::Entity,
        camera: &Camera,
        triggers: &SfxTriggers,
        terrain: &Terrain<TerrainChunk>,
        client: &Client,
    ) {
        let ecs = state.ecs();

        let sfx_event_bus = ecs.read_resource::<EventBus<SfxEventItem>>();
        let mut sfx_emitter = sfx_event_bus.emitter();

        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;

        // Get the player position and chunk
        let player_pos = state
            .read_component_copied::<Pos>(player_entity)
            .unwrap_or_default();
        let player_chunk = player_pos.0.xy().map2(TerrainChunk::RECT_SIZE, |e, sz| {
            (e.floor() as i32).div_euclid(sz as i32)
        });

        // For determining if underground
        let terrain_alt = match client.current_chunk() {
            Some(chunk) => chunk.meta().alt(),
            None => 0.0,
        };

        struct BlockSounds<'a> {
            // The function to select the blocks of interest that we should emit from
            blocks: fn(&'a BlocksOfInterest) -> &'a [Vec3<i32>],
            // The range, in chunks, that the particles should be generated in from the player
            range: usize,
            // The sound of the generated particle
            sfx: SfxEvent,
            // The volume of the sfx
            volume: f32,
            // Condition that must be true to play
            cond: fn(&State) -> bool,
        }

        let sounds: &[BlockSounds] = &[
            BlockSounds {
                blocks: |boi| &boi.leaves,
                range: 1,
                sfx: SfxEvent::Birdcall,
                volume: 1.0,
                cond: |st| st.get_day_period().is_light(),
            },
            BlockSounds {
                blocks: |boi| &boi.leaves,
                range: 1,
                sfx: SfxEvent::Owl,
                volume: 1.0,
                cond: |st| st.get_day_period().is_dark(),
            },
            BlockSounds {
                blocks: |boi| &boi.river,
                range: 1,
                sfx: SfxEvent::RunningWater,
                volume: 1.0,
                cond: |_| true,
            },
            //BlockSounds {
            //    blocks: |boi| &boi.embers,
            //    range: 1,
            //    sfx: SfxEvent::Embers,
            //    volume: 0.15,
            //    //volume: 0.05,
            //    cond: |_| true,
            //    //cond: |st| st.get_day_period().is_dark(),
            //},
            BlockSounds {
                blocks: |boi| &boi.reeds,
                range: 1,
                sfx: SfxEvent::Frog,
                volume: 0.8,
                cond: |st| st.get_day_period().is_dark(),
            },
            //BlockSounds {
            //    blocks: |boi| &boi.flowers,
            //    range: 4,
            //    sfx: SfxEvent::LevelUp,
            //    volume: 1.0,
            //    cond: |st| st.get_day_period().is_dark(),
            //},
            BlockSounds {
                blocks: |boi| &boi.grass,
                range: 1,
                sfx: SfxEvent::Cricket,
                volume: 0.5,
                cond: |st| st.get_day_period().is_dark(),
            },
            BlockSounds {
                blocks: |boi| &boi.beehives,
                range: 1,
                sfx: SfxEvent::Bees,
                volume: 1.0,
                cond: |st| st.get_day_period().is_light(),
            },
        ];

        // Iterate through each kind of block of interest
        for sounds in sounds.iter() {
            // If the timing condition is false, continue
            // or if the player is far enough underground, continue
            if !(sounds.cond)(state) || player_pos.0.z < (terrain_alt - 30.0) {
                continue;
            // Hack to reduce the number of birdcalls (too many leaf blocks)
            } else if (sounds.sfx == SfxEvent::Birdcall || sounds.sfx == SfxEvent::Owl)
                && thread_rng().gen_bool(0.995)
            {
                continue;
            }

            // For chunks surrounding the player position
            for offset in Spiral2d::new().take((sounds.range * 2 + 1).pow(2)) {
                let chunk_pos = player_chunk + offset;

                // Get all the blocks of interest in this chunk
                terrain.get(chunk_pos).map(|chunk_data| {
                    // Get the positions of the blocks of type sounds
                    let blocks = (sounds.blocks)(&chunk_data.blocks_of_interest);

                    let absolute_pos: Vec3<i32> =
                        Vec3::from(chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32));

                    // Iterate through each individual block
                    for block in blocks {
                        // Hack to reduce the number of bird sounds (too many leaf blocks)
                        if (sounds.sfx == SfxEvent::Birdcall || sounds.sfx == SfxEvent::Owl)
                            && thread_rng().gen_bool(0.999)
                        {
                            continue;
                        }
                        let block_pos: Vec3<i32> = absolute_pos + block;
                        let state = self.history.entry(block_pos).or_default();

                        let block_pos = block_pos.map(|x| x as f32);

                        if Self::should_emit(state, triggers.get_key_value(&sounds.sfx)) {
                            // If the camera is within SFX distance
                            if (block_pos.distance_squared(cam_pos)) < SFX_DIST_LIMIT_SQR {
                                sfx_emitter.emit(SfxEventItem::new(
                                    sounds.sfx.clone(),
                                    Some(block_pos),
                                    Some(sounds.volume),
                                ));
                            }
                            state.time = Instant::now();
                            state.event = sounds.sfx.clone();
                        }
                    }
                });
            }
        }
    }
}

impl BlockEventMapper {
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
        }
    }

    fn should_emit(
        previous_state: &PreviousBlockState,
        sfx_trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
    ) -> bool {
        if let Some((event, item)) = sfx_trigger_item {
            if &previous_state.event == event {
                previous_state.time.elapsed().as_secs_f32() >= item.threshold
            } else {
                true
            }
        } else {
            false
        }
    }
}
