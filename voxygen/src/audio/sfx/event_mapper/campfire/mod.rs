/// EventMapper::Campfire maps sfx to campfires
use crate::{
    audio::sfx::{SfxEvent, SfxEventItem, SfxTriggers, SFX_DIST_LIMIT_SQR},
    scene::{Camera, Terrain},
};

use super::EventMapper;

use common::{
    comp::{object, Body, Pos},
    event::EventBus,
    state::State,
    terrain::TerrainChunk,
};
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::time::{Duration, Instant};

pub struct CampfireEventMapper {
    timer: Instant,
}

impl EventMapper for CampfireEventMapper {
    fn maintain(
        &mut self,
        state: &State,
        player_entity: specs::Entity,
        camera: &Camera,
        triggers: &SfxTriggers,
        _terrain: &Terrain<TerrainChunk>,
    ) {
        let ecs = state.ecs();

        let sfx_event_bus = ecs.read_resource::<EventBus<SfxEventItem>>();
        let sfx_emitter = sfx_event_bus.emitter();

        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;
        for (body, pos) in (&ecs.read_storage::<Body>(), &ecs.read_storage::<Pos>()).join() {
            match body {
                Body::Object(object::Body::CampfireLit) => {
                    if (pos.0.distance_squared(cam_pos)) < SFX_DIST_LIMIT_SQR {
                        if self.timer.elapsed().as_secs_f32() > 3.0
                        /* TODO Replace with sensible time */
                        {
                            self.timer = Instant::now();
                            let sfx_trigger_item = triggers.get_trigger(&SfxEvent::LevelUp);
                            if sfx_trigger_item.is_some() {
                                println!("sound");
                                ecs.read_resource::<EventBus<SfxEventItem>>().emit_now(
                                    SfxEventItem::new(
                                        SfxEvent::LevelUp.clone(),
                                        Some(pos.0),
                                        Some(0.0),
                                    ),
                                );
                            }
                        }
                    }
                },
                _ => {},
            }
        }
    }
}

impl CampfireEventMapper {
    pub fn new() -> Self {
        Self {
            timer: Instant::now(),
        }
    }
}
