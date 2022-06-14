use crate::ecs::comp::HpFloaterList;
use common::{
    comp::{Health, Pos},
    resources::{DeltaTime, PlayerEntity},
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, Read, ReadStorage, WriteStorage};

// How long floaters last (in seconds)
pub const HP_SHOWTIME: f32 = 3.0;
pub const CRIT_SHOWTIME: f32 = 0.7;
pub const MY_HP_SHOWTIME: f32 = 2.5;

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, PlayerEntity>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Health>,
        WriteStorage<'a, HpFloaterList>,
    );

    const NAME: &'static str = "floater";
    const ORIGIN: Origin = Origin::Frontend("voxygen");
    const PHASE: Phase = Phase::Create;

    #[allow(clippy::blocks_in_if_conditions)] // TODO: Pending review in #587
    fn run(
        _job: &mut Job<Self>,
        (entities, my_entity, dt, pos, healths, mut hp_floater_lists): Self::SystemData,
    ) {
        // Add hp floater lists to all entities with health and a position
        for entity in (&entities, &healths, &pos, !&hp_floater_lists)
            .join()
            .map(|(e, _, _, _)| e)
            .collect::<Vec<_>>()
        {
            let _ = hp_floater_lists.insert(entity, HpFloaterList {
                floaters: Vec::new(),
                time_since_last_dmg_by_me: None,
            });
        }

        // Remove floater lists on entities without health or without position
        for entity in (&entities, !&healths, &hp_floater_lists)
            .join()
            .map(|(e, _, _)| e)
            .collect::<Vec<_>>()
        {
            hp_floater_lists.remove(entity);
        }
        for entity in (&entities, !&pos, &hp_floater_lists)
            .join()
            .map(|(e, _, _)| e)
            .collect::<Vec<_>>()
        {
            hp_floater_lists.remove(entity);
        }

        // Maintain existing floaters
        for (entity, hp_floater_list) in (&entities, &mut hp_floater_lists).join() {
            // Increment timer for time since last damaged by me
            hp_floater_list
                .time_since_last_dmg_by_me
                .as_mut()
                .map(|t| *t += dt.0);

            for mut floater in hp_floater_list.floaters.iter_mut() {
                // Increment timer
                floater.timer += dt.0;
                floater.jump_timer += dt.0;
            }

            // Clear floaters if newest floater is past show time
            if hp_floater_list.floaters.last().map_or(false, |f| {
                f.timer
                    > if Some(entity) != my_entity.0 {
                        HP_SHOWTIME
                    } else {
                        MY_HP_SHOWTIME
                    }
            }) {
                hp_floater_list.floaters.clear();
            }
        }
    }
}
