use common::{
    comp::{
        skills::{GeneralSkill, Skill},
        Body, CharacterState, Energy, EnergyChange, EnergySource, Health, Poise, PoiseChange,
        PoiseSource, Pos, Stats,
    },
    event::{EventBus, ServerEvent},
    metrics::SysMetrics,
    outcome::Outcome,
    resources::DeltaTime,
    span,
    uid::Uid,
};
use hashbrown::HashSet;
use specs::{
    shred::ResourceId, Entities, Join, Read, ReadExpect, ReadStorage, System, SystemData, World,
    Write, WriteStorage,
};
use vek::Vec3;

const ENERGY_REGEN_ACCEL: f32 = 10.0;
const POISE_REGEN_ACCEL: f32 = 2.0;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    dt: Read<'a, DeltaTime>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    metrics: ReadExpect<'a, SysMetrics>,
    positions: ReadStorage<'a, Pos>,
    uids: ReadStorage<'a, Uid>,
    bodies: ReadStorage<'a, Body>,
    char_states: ReadStorage<'a, CharacterState>,
}

/// This system kills players, levels them up, and regenerates energy.
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, Health>,
        WriteStorage<'a, Poise>,
        WriteStorage<'a, Energy>,
        Write<'a, Vec<Outcome>>,
    );

    fn run(
        &mut self,
        (
            read_data,
            mut stats,
            mut healths,
            mut poises,
            mut energies,
            mut outcomes,
        ): Self::SystemData,
    ) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "stats::Sys::run");
        let mut server_event_emitter = read_data.server_bus.emitter();
        let dt = read_data.dt.0;

        // Increment last change timer
        healths.set_event_emission(false); // avoid unnecessary syncing
        poises.set_event_emission(false); // avoid unnecessary syncing
        for mut health in (&mut healths).join() {
            health.last_change.0 += f64::from(dt);
        }
        for mut poise in (&mut poises).join() {
            poise.last_change.0 += f64::from(dt);
        }
        healths.set_event_emission(true);
        poises.set_event_emission(true);

        // Update stats
        for (entity, uid, mut stats, mut health, pos) in (
            &read_data.entities,
            &read_data.uids,
            &mut stats.restrict_mut(),
            &mut healths.restrict_mut(),
            &read_data.positions,
        )
            .join()
        {
            let set_dead = {
                let health = health.get_unchecked();
                health.should_die() && !health.is_dead
            };

            if set_dead {
                let mut health = health.get_mut_unchecked();
                server_event_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: health.last_change.1.cause,
                });

                health.is_dead = true;
            }

            let stat = stats.get_unchecked();
            let skills_to_level = stat
                .skill_set
                .skill_groups
                .iter()
                .filter_map(|s_g| {
                    (s_g.exp >= stat.skill_set.skill_point_cost(s_g.skill_group_kind))
                        .then(|| s_g.skill_group_kind)
                })
                .collect::<HashSet<_>>();

            if !skills_to_level.is_empty() {
                let mut stat = stats.get_mut_unchecked();
                for skill_group in skills_to_level {
                    stat.skill_set.earn_skill_point(skill_group);
                    outcomes.push(Outcome::SkillPointGain {
                        uid: *uid,
                        skill_tree: skill_group,
                        total_points: stat.skill_set.earned_sp(skill_group),
                        pos: pos.0,
                    });
                }
            }
        }

        // Apply effects from leveling skills
        for (mut stats, mut health, mut energy, body) in (
            &mut stats.restrict_mut(),
            &mut healths.restrict_mut(),
            &mut energies.restrict_mut(),
            &read_data.bodies,
        )
            .join()
        {
            let stat = stats.get_unchecked();
            if stat.skill_set.modify_health {
                let mut health = health.get_mut_unchecked();
                let health_level = stat
                    .skill_set
                    .skill_level(Skill::General(GeneralSkill::HealthIncrease))
                    .unwrap_or(None)
                    .unwrap_or(0);
                health.update_max_hp(Some(*body), health_level);
                let mut stat = stats.get_mut_unchecked();
                stat.skill_set.modify_health = false;
            }
            let stat = stats.get_unchecked();
            if stat.skill_set.modify_energy {
                let mut energy = energy.get_mut_unchecked();
                let energy_level = stat
                    .skill_set
                    .skill_level(Skill::General(GeneralSkill::EnergyIncrease))
                    .unwrap_or(None)
                    .unwrap_or(0);
                energy.update_max_energy(Some(*body), energy_level);
                let mut stat = stats.get_mut_unchecked();
                stat.skill_set.modify_energy = false;
            }
        }

        // Update energies and poises
        for (character_state, mut energy, mut poise) in (
            &read_data.char_states,
            &mut energies.restrict_mut(),
            &mut poises.restrict_mut(),
        )
            .join()
        {
            match character_state {
                // Accelerate recharging energy.
                CharacterState::Idle { .. }
                | CharacterState::Talk { .. }
                | CharacterState::Sit { .. }
                | CharacterState::Dance { .. }
                | CharacterState::Sneak { .. }
                | CharacterState::GlideWield { .. }
                | CharacterState::Wielding { .. }
                | CharacterState::Equipping { .. }
                | CharacterState::Boost { .. } => {
                    let res = {
                        let energy = energy.get_unchecked();
                        energy.current() < energy.maximum()
                    };

                    if res {
                        let mut energy = energy.get_mut_unchecked();
                        let energy = &mut *energy;
                        // Have to account for Calc I differential equations due to acceleration
                        energy.change_by(EnergyChange {
                            amount: (energy.regen_rate * dt + ENERGY_REGEN_ACCEL * dt.powi(2) / 2.0)
                                as i32,
                            source: EnergySource::Regen,
                        });
                        energy.regen_rate =
                            (energy.regen_rate + ENERGY_REGEN_ACCEL * dt).min(100.0);
                    }

                    let res_poise = {
                        let poise = poise.get_unchecked();
                        poise.current() < poise.maximum()
                    };

                    if res_poise {
                        let mut poise = poise.get_mut_unchecked();
                        let poise = &mut *poise;
                        poise.change_by(
                            PoiseChange {
                                amount: (poise.regen_rate * dt
                                    + POISE_REGEN_ACCEL * dt.powi(2) / 2.0)
                                    as i32,
                                source: PoiseSource::Regen,
                            },
                            Vec3::zero(),
                        );
                        poise.regen_rate = (poise.regen_rate + POISE_REGEN_ACCEL * dt).min(10.0);
                    }
                },
                // Ability and glider use does not regen and sets the rate back to zero.
                CharacterState::Glide { .. }
                | CharacterState::BasicMelee { .. }
                | CharacterState::DashMelee { .. }
                | CharacterState::LeapMelee { .. }
                | CharacterState::SpinMelee { .. }
                | CharacterState::ComboMelee { .. }
                | CharacterState::BasicRanged { .. }
                | CharacterState::ChargedMelee { .. }
                | CharacterState::ChargedRanged { .. }
                | CharacterState::RepeaterRanged { .. }
                | CharacterState::Shockwave { .. }
                | CharacterState::BasicBeam { .. } => {
                    if energy.get_unchecked().regen_rate != 0.0 {
                        energy.get_mut_unchecked().regen_rate = 0.0
                    }
                },
                // recover small amount of passive energy from blocking, and bonus energy from
                // blocking attacks?
                CharacterState::BasicBlock => {
                    let res = {
                        let energy = energy.get_unchecked();
                        energy.current() < energy.maximum()
                    };

                    if res {
                        energy.get_mut_unchecked().change_by(EnergyChange {
                            amount: -3,
                            source: EnergySource::Regen,
                        });
                    }
                },
                // Non-combat abilities that consume energy;
                // temporarily stall energy gain, but preserve regen_rate.
                CharacterState::Roll { .. }
                | CharacterState::Climb { .. }
                | CharacterState::Stunned { .. } => {},
            }
        }

        read_data.metrics.stats_ns.store(
            start_time.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
