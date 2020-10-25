/// EventMapper::Block watches the sound emitting blocks in the same
/// chunk as the player and emits ambient sfx
use crate::{
    audio::sfx::{SfxEvent, SfxEventItem, SfxTriggers, SFX_DIST_LIMIT_SQR},
    scene::{terrain::BlocksOfInterest, Camera, Terrain},
};

use super::EventMapper;
use common::{
    comp::Pos, event::EventBus, spiral::Spiral2d, state::State, terrain::TerrainChunk,
    vol::RectRasterableVol,
};
use rand::{prelude::SliceRandom, thread_rng};
use specs::{Join, WorldExt};
use std::time::Instant;
use vek::*;

enum BlockEmitter {
    Leaves,
    Grass,
    Embers,
    Beehives,
    Reeds,
    Flowers,
}

pub struct BlockEventMapper {
    timer: Instant,
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

        //for (entity, pos) in (&ecs.entities(), &ecs.read_storage::<Pos>())
        //    .join()
        //    .filter(|(_, e_pos, ..)| (e_pos.0.distance_squared(cam_pos)) <
        // SFX_DIST_LIMIT_SQR)
        //{
        //}

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
            // The emission rate, per block per second, of the generated particles
            rate: f32,
            // The number of seconds that each particle should live for
            lifetime: f32,
            // The sound of the generated particle
            sound: SfxEvent,
            // Condition that must be true
            cond: fn(&State) -> bool,
        }
        let sounds: &[BlockSounds] = &[
            BlockSounds {
                blocks: |boi| &boi.leaves,
                range: 4,
                rate: 0.1,
                lifetime: 3.0,
                sound: SfxEvent::LevelUp,
                cond: |_| true,
            },
            BlockSounds {
                blocks: |boi| &boi.embers,
                range: 2,
                rate: 0.5,
                lifetime: 2.25,
                sound: SfxEvent::Roll,
                cond: |_| true,
            },
            BlockSounds {
                blocks: |boi| &boi.reeds,
                range: 6,
                rate: 0.4,
                lifetime: 4.0,
                sound: SfxEvent::Roll,
                cond: |st| st.get_day_period().is_dark(),
            },
            BlockSounds {
                blocks: |boi| &boi.flowers,
                range: 5,
                rate: 0.2,
                lifetime: 4.0,
                sound: SfxEvent::Roll,
                cond: |st| st.get_day_period().is_dark(),
            },
            BlockSounds {
                blocks: |boi| &boi.beehives,
                range: 3,
                rate: 0.5,
                lifetime: 3.0,
                sound: SfxEvent::Roll,
                cond: |st| st.get_day_period().is_light(),
            },
        ];
        let mut rng = thread_rng();
        for sounds in sounds.iter() {
            if !(sounds.cond)(state) {
                continue;
            }
            for offset in Spiral2d::new().take((sounds.range * 2 + 1).pow(2)) {
                let chunk_pos = player_chunk + offset;

                terrain.get(chunk_pos).map(|chunk_data| {
                    let blocks = (sounds.blocks)(&chunk_data.blocks_of_interest);
                    let block_pos: Vec3<i32> =
                        Vec3::from(chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32))
                            + blocks
                                .choose(&mut rng)
                                .copied()
                                .unwrap_or_else(|| Vec3::new(0, 0, 0));

                    let block_pos = Vec3::new(
                        block_pos[0] as f32,
                        block_pos[1] as f32,
                        block_pos[2] as f32,
                    );

                    if (block_pos.distance_squared(cam_pos)) < SFX_DIST_LIMIT_SQR {
                        if let Some(mapped_event) = self.map_event(BlockEmitter::Leaves) {
                            let sfx_trigger_item = triggers.get_trigger(&mapped_event);
                            if sfx_trigger_item.is_some() {
                                println!("sound");
                                ecs.read_resource::<EventBus<SfxEventItem>>().emit_now(
                                    SfxEventItem::new(
                                        mapped_event.clone(),
                                        Some(block_pos),
                                        Some(1.0),
                                    ),
                                );
                            }
                        }
                    }
                });
            }
        }
    }
    //let leaves_pos = BlocksOfInterest::from_chunk(&player_chunk).leaves;
    //if leaves_pos.len() > 0 {
    //    let my_leaf_pos = Vec3::new(
    //        leaves_pos[0][0] as f32,
    //        leaves_pos[0][1] as f32,
    //        leaves_pos[0][2] as f32,
    //    );
    //    println!("my leaf pos: {:?}", my_leaf_pos);

    //    if let Some(mapped_event) = self.map_event(BlockEmitter::Leaves) {
    //        let sfx_trigger_item = triggers.get_trigger(&mapped_event);

    //        if leaves_pos.len() > 0 {
    //            println!("Num leaves: {:?}", leaves_pos.len());
    //        }
    //        //for i in 0..leaves_pos.len() {
    //        //    if i < 5 {
    //        if sfx_trigger_item.is_some() {
    //            println!("sound");
    //            ecs.read_resource::<EventBus<SfxEventItem>>()
    //                .emit_now(SfxEventItem::new(
    //                    mapped_event.clone(),
    //                    Some(my_leaf_pos),
    //                    Some(1.0),
    //                ));
    //        }
    //        //    }
    //        //}
    //    }
    //}
}

impl BlockEventMapper {
    pub fn new() -> Self {
        Self {
            timer: Instant::now(),
        }
    }

    fn map_event(&mut self, blocktype: BlockEmitter) -> Option<SfxEvent> {
        if self.timer.elapsed().as_secs_f32() > 1.0 {
            self.timer = Instant::now();
            let sfx_event = match blocktype {
                BlockEmitter::Leaves => Some(SfxEvent::LevelUp),
                BlockEmitter::Grass => Some(SfxEvent::Roll),
                BlockEmitter::Embers => Some(SfxEvent::Roll),
                BlockEmitter::Beehives => Some(SfxEvent::Roll),
                BlockEmitter::Reeds => Some(SfxEvent::Roll),
                BlockEmitter::Flowers => Some(SfxEvent::Roll),
            };

            sfx_event
        } else {
            None
        }
    }
}
