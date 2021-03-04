use crate::ecs::{
    comp::{HpFloater, HpFloaterList},
    MyEntity,
};
use common::{
    comp::{Health, HealthSource, Pos},
    resources::DeltaTime,
    uid::Uid,
    vsystem::{Origin, Phase, VJob, VSystem},
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, WriteStorage};

// How long floaters last (in seconds)
pub const HP_SHOWTIME: f32 = 3.0;
pub const MY_HP_SHOWTIME: f32 = 2.5;
pub const HP_ACCUMULATETIME: f32 = 1.0;

#[derive(Default)]
pub struct Sys;
impl<'a> VSystem<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, MyEntity>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Health>,
        WriteStorage<'a, HpFloaterList>,
    );

    const NAME: &'static str = "floater";
    const ORIGIN: Origin = Origin::Frontend("voxygen");
    const PHASE: Phase = Phase::Create;

    #[allow(clippy::blocks_in_if_conditions)] // TODO: Pending review in #587
    fn run(
        _job: &mut VJob<Self>,
        (entities, my_entity, dt, uids, pos, healths, mut hp_floater_lists): Self::SystemData,
    ) {
        // Add hp floater lists to all entities with health and a position
        // Note: necessary in order to know last_hp
        for (entity, last_hp) in (&entities, &healths, &pos, !&hp_floater_lists)
            .join()
            .map(|(e, h, _, _)| (e, h.current()))
            .collect::<Vec<_>>()
        {
            let _ = hp_floater_lists.insert(entity, HpFloaterList {
                floaters: Vec::new(),
                last_hp,
                time_since_last_dmg_by_me: None,
            });
        }

        // Add hp floaters to all entities that have been damaged
        let my_uid = uids.get(my_entity.0);
        for (entity, health, hp_floater_list) in (&entities, &healths, &mut hp_floater_lists).join()
        {
            // Increment timer for time since last damaged by me
            hp_floater_list
                .time_since_last_dmg_by_me
                .as_mut()
                .map(|t| *t += dt.0);

            // Check if health has changed (won't work if damaged and then healed with
            // equivalently in the same frame)
            if hp_floater_list.last_hp != health.current() {
                hp_floater_list.last_hp = health.current();
                // TODO: What if multiple health changes occurred since last check here
                // Also, If we make health store a vec of the last_changes (from say the last
                // frame), what if the client receives the health component from
                // two different server ticks at once, then one will be lost
                // (tbf this is probably a rare occurance and the results
                // would just be a transient glitch in the display of these damage numbers)
                // (maybe health changes could be sent to the client as a list
                // of events)
                if match health.last_change.1.cause {
                    HealthSource::Damage { by: Some(by), .. }
                    | HealthSource::Heal { by: Some(by) } => {
                        let by_me = my_uid.map_or(false, |&uid| by == uid);
                        // If the attack was by me also reset this timer
                        if by_me {
                            hp_floater_list.time_since_last_dmg_by_me = Some(0.0);
                        }
                        my_entity.0 == entity || by_me
                    },
                    HealthSource::Suicide => my_entity.0 == entity,
                    HealthSource::World => my_entity.0 == entity,
                    HealthSource::LevelUp => my_entity.0 == entity,
                    HealthSource::Command => true,
                    HealthSource::Item => true,
                    _ => false,
                } {
                    let last_floater = hp_floater_list.floaters.last_mut();
                    match last_floater {
                        Some(f) if f.timer < HP_ACCUMULATETIME => {
                            //TODO: Add "jumping" animation on floater when it changes its value
                            f.hp_change += health.last_change.1.amount;
                        },
                        _ => {
                            hp_floater_list.floaters.push(HpFloater {
                                timer: 0.0,
                                hp_change: health.last_change.1.amount,
                                rand: rand::random(),
                            });
                        },
                    }
                }
            }
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
        for (
            entity,
            HpFloaterList {
                ref mut floaters,
                ref last_hp,
                ..
            },
        ) in (&entities, &mut hp_floater_lists).join()
        {
            for mut floater in floaters.iter_mut() {
                // Increment timer
                floater.timer += dt.0;
            }
            // Clear floaters if newest floater is past show time or health runs out
            if floaters.last().map_or(false, |f| {
                f.timer
                    > if entity != my_entity.0 {
                        HP_SHOWTIME
                    } else {
                        MY_HP_SHOWTIME
                    }
                    || *last_hp == 0
            }) {
                floaters.clear();
            }
        }
    }
}
