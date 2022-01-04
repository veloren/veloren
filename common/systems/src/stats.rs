use common::{
    combat,
    comp::{
        self,
        skills::{GeneralSkill, Skill},
        Body, CharacterState, Combo, Energy, Health, Inventory, Poise, Pos, SkillSet, Stats,
        StatsModifier,
    },
    event::{EventBus, ServerEvent},
    resources::{DeltaTime, EntitiesDiedLastTick, Time},
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{
    shred::ResourceId, Entities, Join, Read, ReadStorage, SystemData, World, Write, WriteStorage,
};
use vek::Vec3;

const ENERGY_REGEN_ACCEL: f32 = 1.0;
const POISE_REGEN_ACCEL: f32 = 2.0;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    dt: Read<'a, DeltaTime>,
    time: Read<'a, Time>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    positions: ReadStorage<'a, Pos>,
    bodies: ReadStorage<'a, Body>,
    char_states: ReadStorage<'a, CharacterState>,
    inventories: ReadStorage<'a, Inventory>,
}

/// This system kills players, levels them up, and regenerates energy.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, SkillSet>,
        WriteStorage<'a, Health>,
        WriteStorage<'a, Poise>,
        WriteStorage<'a, Energy>,
        WriteStorage<'a, Combo>,
        Write<'a, EntitiesDiedLastTick>,
    );

    const NAME: &'static str = "stats";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            read_data,
            stats,
            mut skill_sets,
            mut healths,
            mut poises,
            mut energies,
            mut combos,
            mut entities_died_last_tick,
        ): Self::SystemData,
    ) {
        entities_died_last_tick.0.clear();
        let mut server_event_emitter = read_data.server_bus.emitter();
        let dt = read_data.dt.0;

        // Update stats
        for (entity, stats, mut health, pos, mut energy, inventory) in (
            &read_data.entities,
            &stats,
            &mut healths,
            &read_data.positions,
            &mut energies,
            read_data.inventories.maybe(),
        )
            .join()
        {
            let set_dead = { health.should_die() && !health.is_dead };

            if set_dead {
                let cloned_entity = (entity, *pos);
                entities_died_last_tick.0.push(cloned_entity);
                server_event_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: health.last_change,
                });

                health.is_dead = true;
            }
            let stat = stats;

            let update_max_hp = {
                stat.max_health_modifiers.update_maximum()
                    || (health.base_max() - health.maximum()).abs() > Health::HEALTH_EPSILON
            };

            if update_max_hp {
                health.update_maximum(stat.max_health_modifiers);
            }

            let (change_energy, energy_mods) = {
                // Calculates energy scaling from stats and inventory
                let energy_mods = StatsModifier {
                    add_mod: stat.max_energy_modifiers.add_mod
                        + combat::compute_max_energy_mod(inventory),
                    mult_mod: stat.max_energy_modifiers.mult_mod,
                };
                (
                    energy_mods.update_maximum()
                        || (energy.base_max() - energy.maximum()).abs() > Energy::ENERGY_EPSILON,
                    energy_mods,
                )
            };

            // If modifier sufficiently different, mutably access energy
            if change_energy {
                energy.update_maximum(energy_mods);
            }
        }

        // Apply effects from leveling skills
        for (mut skill_set, mut health, mut energy, body) in (
            &mut skill_sets,
            &mut healths,
            &mut energies,
            &read_data.bodies,
        )
            .join()
        {
            if skill_set.modify_health {
                let health_level = skill_set
                    .skill_level(Skill::General(GeneralSkill::HealthIncrease))
                    .unwrap_or(0);
                health.update_max_hp(*body, health_level);
                skill_set.modify_health = false;
            }
            if skill_set.modify_energy {
                let energy_level = skill_set
                    .skill_level(Skill::General(GeneralSkill::EnergyIncrease))
                    .unwrap_or(0);
                energy.update_max_energy(*body, energy_level);
                skill_set.modify_energy = false;
            }
        }

        // Update energies and poises
        for (character_state, mut energy, mut poise) in
            (&read_data.char_states, &mut energies, &mut poises).join()
        {
            match character_state {
                // Accelerate recharging energy.
                CharacterState::Idle { .. }
                | CharacterState::Talk { .. }
                | CharacterState::Sit { .. }
                | CharacterState::Dance { .. }
                | CharacterState::Glide { .. }
                | CharacterState::GlideWield { .. }
                | CharacterState::Wielding { .. }
                | CharacterState::Equipping { .. }
                | CharacterState::Boost { .. } => {
                    let res = { energy.current() < energy.maximum() };

                    if res {
                        let energy = &mut *energy;
                        energy.change_by(energy.regen_rate * dt);
                        energy.regen_rate = (energy.regen_rate + ENERGY_REGEN_ACCEL * dt).min(10.0);
                    }

                    let res_poise = { poise.current() < poise.maximum() };

                    if res_poise {
                        let poise = &mut *poise;
                        poise.change_by(poise.regen_rate * dt, Vec3::zero());
                        poise.regen_rate = (poise.regen_rate + POISE_REGEN_ACCEL * dt).min(10.0);
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
                | CharacterState::BasicBeam { .. }
                | CharacterState::BasicAura { .. }
                | CharacterState::Blink { .. }
                | CharacterState::BasicSummon { .. }
                | CharacterState::SelfBuff { .. }
                | CharacterState::SpriteSummon { .. } => {
                    if energy.regen_rate != 0.0 {
                        energy.regen_rate = 0.0
                    }
                },
                // Abilities that temporarily stall energy gain, but preserve regen_rate.
                CharacterState::Roll { .. }
                | CharacterState::Climb { .. }
                | CharacterState::Stunned { .. }
                | CharacterState::BasicBlock { .. }
                | CharacterState::UseItem { .. }
                | CharacterState::SpriteInteract { .. } => {},
            }
        }

        // Decay combo
        for (_, mut combo) in (&read_data.entities, &mut combos).join() {
            if combo.counter() > 0
                && read_data.time.0 - combo.last_increase() > comp::combo::COMBO_DECAY_START
            {
                combo.reset();
            }
        }
    }
}
