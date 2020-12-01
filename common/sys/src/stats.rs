use common::{
    comp::{CharacterState, Energy, EnergyChange, EnergySource, Health, HealthSource, Stats},
    event::{EventBus, ServerEvent},
    metrics::SysMetrics,
    resources::DeltaTime,
    span,
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};

const ENERGY_REGEN_ACCEL: f32 = 10.0;

/// This system kills players, levels them up, and regenerates energy.
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        ReadExpect<'a, SysMetrics>,
        ReadStorage<'a, CharacterState>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, Health>,
        WriteStorage<'a, Energy>,
    );

    fn run(
        &mut self,
        (
            entities,
            dt,
            server_event_bus,
            sys_metrics,
            character_states,
            mut stats,
            mut healths,
            mut energies,
        ): Self::SystemData,
    ) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "stats::Sys::run");
        let mut server_event_emitter = server_event_bus.emitter();

        // Increment last change timer
        healths.set_event_emission(false); // avoid unnecessary syncing
        for health in (&mut healths).join() {
            health.last_change.0 += f64::from(dt.0);
        }
        healths.set_event_emission(true);

        // Update stats
        for (entity, mut stats, mut health) in (
            &entities,
            &mut stats.restrict_mut(),
            &mut healths.restrict_mut(),
        )
            .join()
        {
            let (set_dead, level_up) = {
                let stat = stats.get_unchecked();
                let health = health.get_unchecked();
                (
                    health.should_die() && !health.is_dead,
                    stat.exp.current() >= stat.exp.maximum(),
                )
            };

            if set_dead {
                let health = health.get_mut_unchecked();
                server_event_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: health.last_change.1.cause,
                });

                health.is_dead = true;
            }

            if level_up {
                let stat = stats.get_mut_unchecked();
                while stat.exp.current() >= stat.exp.maximum() {
                    stat.exp.change_by(-(stat.exp.maximum() as i64));
                    stat.level.change_by(1);
                    stat.exp.update_maximum(stat.level.level());
                    server_event_emitter.emit(ServerEvent::LevelUp(entity, stat.level.level()));
                }

                let health = health.get_mut_unchecked();
                health.update_max_hp(Some(stat.body_type), stat.level.level());
                health.set_to(health.maximum(), HealthSource::LevelUp);
            }
        }

        // Update energies
        for (character_state, mut energy) in
            (&character_states, &mut energies.restrict_mut()).join()
        {
            match character_state {
                // Accelerate recharging energy.
                CharacterState::Idle { .. }
                | CharacterState::Sit { .. }
                | CharacterState::Dance { .. }
                | CharacterState::Sneak { .. }
                | CharacterState::Glide { .. }
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
                        // Have to account for Calc I differential equations due to acceleration
                        energy.change_by(EnergyChange {
                            amount: (energy.regen_rate * dt.0
                                + ENERGY_REGEN_ACCEL * dt.0.powi(2) / 2.0)
                                as i32,
                            source: EnergySource::Regen,
                        });
                        energy.regen_rate =
                            (energy.regen_rate + ENERGY_REGEN_ACCEL * dt.0).min(100.0);
                    }
                },
                // Ability use does not regen and sets the rate back to zero.
                CharacterState::BasicMelee { .. }
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
                CharacterState::Roll { .. } | CharacterState::Climb { .. } => {},
            }
        }
        sys_metrics.stats_ns.store(
            start_time.elapsed().as_nanos() as i64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
