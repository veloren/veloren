use crate::ecs::{
    comp::{HpFloater, HpFloaterList},
    ExpFloater, MyEntity, MyExpFloaterList,
};
use common::{
    comp::{HealthSource, Pos, Stats},
    state::DeltaTime,
    sync::Uid,
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteStorage};

// How long floaters last (in seconds)
pub const HP_SHOWTIME: f32 = 3.0;
pub const MY_HP_SHOWTIME: f32 = 2.5;
pub const MY_EXP_SHOWTIME: f32 = 4.0;

pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, MyEntity>,
        Read<'a, DeltaTime>,
        Write<'a, MyExpFloaterList>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Stats>,
        WriteStorage<'a, HpFloaterList>,
    );

    fn run(
        &mut self,
        (entities, my_entity, dt, mut my_exp_floater_list, uids, pos, stats, mut hp_floater_lists): Self::SystemData,
    ) {
        // Add hp floater lists to all entities with stats and a postion
        // Note: neccessary in order to know last_hp
        for (entity, last_hp) in (&entities, &stats, &pos, !&hp_floater_lists)
            .join()
            .map(|(e, s, _, _)| (e, s.health.current()))
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
        for (entity, health, hp_floater_list) in (&entities, &stats, &mut hp_floater_lists)
            .join()
            .map(|(e, s, fl)| (e, s.health, fl))
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
                // TODO: What if multiple health changes occured since last check here
                // Also, If we make stats store a vec of the last_changes (from say the last
                // frame), what if the client recieves the stats component from
                // two different server ticks at once, then one will be lost
                // (tbf this is probably a rare occurance and the results
                // would just be a transient glitch in the display of these damage numbers)
                // (maybe health changes could be sent to the client as a list
                // of events)
                if match health.last_change.1.cause {
                    HealthSource::Attack { by } | HealthSource::Projectile { owner: Some(by) } => {
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
                    hp_floater_list.floaters.push(HpFloater {
                        timer: 0.0,
                        hp_change: health.last_change.1.amount,
                        rand: rand::random(),
                    });
                }
            }
        }

        // Remove floater lists on entities without stats or without posistion
        for entity in (&entities, !&stats, &hp_floater_lists)
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

        // Update MyExpFloaterList
        if let Some(stats) = stats.get(my_entity.0) {
            let mut fl = my_exp_floater_list;
            // Add a floater if exp changed
            // TODO: can't handle if you level up more than once (maybe store total exp in
            // stats)
            let exp_change = if stats.level.level() != fl.last_level {
                if stats.level.level() > fl.last_level {
                    stats.exp.current() as i32 + fl.last_exp_max as i32 - fl.last_exp as i32
                } else {
                    // Level down
                    stats.exp.current() as i32 - stats.exp.maximum() as i32 - fl.last_exp as i32
                }
            } else {
                stats.exp.current() as i32 - fl.last_exp as i32
            };

            if exp_change != 0 {
                fl.floaters.push(ExpFloater {
                    timer: 0.0,
                    exp_change,
                    rand: (rand::random(), rand::random()),
                });
            }

            // Increment timers
            for mut floater in &mut fl.floaters {
                floater.timer += dt.0;
            }

            // Clear if the newest is past show time
            if fl
                .floaters
                .last()
                .map_or(false, |f| f.timer > MY_EXP_SHOWTIME)
            {
                fl.floaters.clear();
            }

            // Update stored values
            fl.last_exp = stats.exp.current();
            fl.last_exp_max = stats.exp.maximum();
            fl.last_level = stats.level.level();
        } else {
            // Clear if stats component doesn't exist
            my_exp_floater_list.floaters.clear();
            // Clear stored values
            my_exp_floater_list.last_exp = 0;
            my_exp_floater_list.last_exp_max = 0;
            my_exp_floater_list.last_level = 0;
        }
    }
}
