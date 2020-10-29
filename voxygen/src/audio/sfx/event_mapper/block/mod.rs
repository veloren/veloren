/// EventMapper::Block watches the sound emitting blocks in the same
/// chunk as the player and emits ambient sfx
use crate::{
    audio::sfx::{SfxEvent, SfxEventItem, SfxTriggerItem, SfxTriggers, SFX_DIST_LIMIT_SQR},
    scene::{terrain::BlocksOfInterest, Camera, Terrain},
};

use super::EventMapper;
use common::{
    comp::Pos, event::EventBus, spiral::Spiral2d, state::State, terrain::TerrainChunk,
    vol::RectRasterableVol,
};
use hashbrown::HashMap;
use rand::{prelude::SliceRandom, thread_rng, Rng};
use specs::WorldExt;
use std::time::Instant;
use vek::*;

//enum BlockEmitter {
//    Leaves,
//    Grass,
//    Embers,
//    Beehives,
//    Reeds,
//    Flowers,
//}

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

impl PreviousBlockState {
    fn new(event: SfxEvent) -> Self {
        PreviousBlockState {
            event,
            time: Instant::now(),
        }
    }
}

pub struct BlockEventMapper {
    timer: Instant,
    counter: usize,
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
    ) {
        let ecs = state.ecs();

        let sfx_event_bus = ecs.read_resource::<EventBus<SfxEventItem>>();
        let mut sfx_emitter = sfx_event_bus.emitter();

        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;

        let player_pos = state
            .read_component_copied::<Pos>(player_entity)
            .unwrap_or_default();
        let player_chunk = player_pos.0.xy().map2(TerrainChunk::RECT_SIZE, |e, sz| {
            (e.floor() as i32).div_euclid(sz as i32)
        });

        struct BlockSounds<'a> {
            // The function to select the blocks of interest that we should emit from
            blocks: fn(&'a BlocksOfInterest) -> &'a [Vec3<i32>],
            // The range, in chunks, that the particles should be generated in from the player
            range: usize,
            // The sound of the generated particle
            sfx: SfxEvent,
            // The volume of the sfx
            volume: f32,
            // Condition that must be true
            cond: fn(&State) -> bool,
        }
        let sounds: &[BlockSounds] = &[
            BlockSounds {
                blocks: |boi| &boi.leaves,
                range: 1,
                sfx: SfxEvent::Birdcall,
                volume: 1.0,
                cond: |_| true,
            },
            BlockSounds {
                blocks: |boi| &boi.embers,
                range: 1,
                sfx: SfxEvent::Embers,
                volume: 0.05,
                //volume: 0.05,
                cond: |_| true,
                //cond: |st| st.get_day_period().is_dark(),
            },
            //BlockSounds {
            //    blocks: |boi| &boi.reeds,
            //    range: 4,
            //    spacing: 2.0,
            //    sfx: SfxEvent::Run,
            //    volume: 1.0,
            //    //cond: |st| st.get_day_period().is_dark(),
            //    cond: |_| true,
            //},
            //BlockSounds {
            //    blocks: |boi| &boi.flowers,
            //    range: 4,
            //    spacing: 2.5,
            //    sfx: SfxEvent::LevelUp,
            //    volume: 1.0,
            //    cond: |st| st.get_day_period().is_dark(),
            //},
            //BlockSounds {
            //    blocks: |boi| &boi.grass,
            //    range: 4,
            //    spacing: 2.5,
            //    sfx: SfxEvent::Roll,
            //    volume: 1.0,
            //    //cond: |st| st.get_day_period().is_light(),
            //    cond: |_| false,
            //},
            //BlockSounds {
            //    blocks: |boi| &boi.beehives,
            //    range: 4,
            //    spacing: 1.5,
            //    sfx: SfxEvent::Roll,
            //    volume: 1.0,
            //    //cond: |st| st.get_day_period().is_light(),
            //    cond: |_| true,
            //},
        ];

        // Iterate through each kind of block of interest
        for sounds in sounds.iter() {
            if !(sounds.cond)(state) {
                continue;
            }

            // For chunks surrounding the player position
            for offset in Spiral2d::new().take((sounds.range * 2 + 1).pow(2)) {
                let chunk_pos = player_chunk + offset;

                // Get all the blocks of interest in this chunk
                terrain.get(chunk_pos).map(|chunk_data| {
                    // Get all the blocks of type sounds
                    let blocks = (sounds.blocks)(&chunk_data.blocks_of_interest);

                    let absolute_pos: Vec3<i32> =
                        Vec3::from(chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32));

                    // Iterate through each individual block
                    for block in blocks {
                        // Reduce the number of bird calls from trees
                        if sounds.sfx == SfxEvent::Birdcall && thread_rng().gen::<f32>() < 0.25 {
                            continue;
                        }

                        let block_pos: Vec3<i32> = absolute_pos + block;
                        let state = self.history.entry(block_pos).or_default();

                        // Convert to f32 for sfx emitter
                        let block_pos = Vec3::new(
                            block_pos[0] as f32,
                            block_pos[1] as f32,
                            block_pos[2] as f32,
                        );

                        if Self::should_emit(state, triggers.get_key_value(&sounds.sfx)) {
                            // If the camera is within SFX distance
                            if (block_pos.distance_squared(cam_pos)) < SFX_DIST_LIMIT_SQR {
                                sfx_emitter.emit(SfxEventItem::new(sounds.sfx.clone(), Some(block_pos), Some(sounds.volume)));
                                state.time = Instant::now();
                                state.event = sounds.sfx.clone();
                            }
                        }
                    }

                        //// If the timer for this block is over the spacing
                        //// and the block is in the history
                        //if self.history.contains_key(&block_pos) {
                        //    if self
                        //        .history
                        //        .get(&block_pos)
                        //        .unwrap() // can't fail as the key is in the hashmap
                        //        .elapsed()
                        //        .as_secs_f32()
                        //        > sounds.spacing
                        //    {
                        //        // Reset timer for this block
                        //        self.history.insert(block_pos, Instant::now());

                        //        // Convert to f32 for distance_squared function
                        //        let block_pos = Vec3::new(
                        //            block_pos[0] as f32,
                        //            block_pos[1] as f32,
                        //            block_pos[2] as f32,
                        //        );

                        //        // If the camera is within SFX distance
                        //        if (block_pos.distance_squared(cam_pos)) < SFX_DIST_LIMIT_SQR {
                        //            // Emit the sound
                        //            let sfx_trigger_item = triggers.get_trigger(&sounds.sfx);
                        //            if sfx_trigger_item.is_some() {
                        //                ecs.read_resource::<EventBus<SfxEventItem>>().emit_now(
                        //                    SfxEventItem::new(
                        //                        sounds.sfx.clone(),
                        //                        Some(block_pos),
                        //                        Some(sounds.volume),
                        //                    ),
                        //                );
                        //            }
                        //        }
                        //    }
                        //} else {
                        //    // Start the timer for this block
                        //    self.history.insert(block_pos, Instant::now());
                        //}
                    //}
                });
            }
        }
    }
}

impl BlockEventMapper {
    pub fn new() -> Self {
        Self {
            timer: Instant::now(),
            counter: 0,
            history: HashMap::new(),
        }
    }

    fn should_emit(
        previous_state: &PreviousBlockState,
        sfx_trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
    ) -> bool {
        if let Some((event, item)) = sfx_trigger_item {
            if &previous_state.event == event {
                previous_state.time.elapsed().as_secs_f64() >= item.threshold
            } else {
                true
            }
        } else {
            false
        }
    }
    //fn map_event(&mut self, blocktype: BlockEmitter) -> Option<SfxEvent> {
    //    if self.timer.elapsed().as_secs_f32() > 1.0 {
    //        self.timer = Instant::now();
    //        let sfx_event = match blocktype {
    //            BlockEmitter::Leaves => Some(SfxEvent::LevelUp),
    //            BlockEmitter::Grass => Some(SfxEvent::Roll),
    //            BlockEmitter::Embers => Some(SfxEvent::Roll),
    //            BlockEmitter::Beehives => Some(SfxEvent::Roll),
    //            BlockEmitter::Reeds => Some(SfxEvent::Roll),
    //            BlockEmitter::Flowers => Some(SfxEvent::Roll),
    //        };

    //        sfx_event
    //    } else {
    //        None
    //    }
    //}
}
