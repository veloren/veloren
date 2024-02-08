use crate::wiring::{Circuit, WiringElement};
use common::{
    comp::{LightEmitter, PhysicsState, Pos},
    event, event_emitters,
    resources::EntitiesDiedLastTick,
};
use common_ecs::{Job, Origin, Phase, System};
use common_state::BlockChange;
use hashbrown::HashMap;
use specs::{
    shred, Entities, Entity, Join, LendJoin, Read, ReadStorage, SystemData, Write, WriteStorage,
};

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    circuits: ReadStorage<'a, Circuit>,
    pos: ReadStorage<'a, Pos>,
    physics_states: ReadStorage<'a, PhysicsState>,
    entities_died_last_tick: Read<'a, EntitiesDiedLastTick>,
}

event_emitters! {
    struct Events[Emitters] {
        shoot: event::ShootEvent,
    }
}

/// This system is responsible for handling wiring (signals and wiring systems)
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        Events<'a>,
        WriteStorage<'a, WiringElement>,
        WriteStorage<'a, LightEmitter>, // maybe
        Write<'a, BlockChange>,
    );

    const NAME: &'static str = "wiring";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (read_data, events, mut wiring_elements, mut light_emitters, mut block_change): Self::SystemData,
    ) {
        let mut emitters = events.get_emitters();

        // Compute the output for each wiring element by computing
        // the output for each `OutputFormula` and store each value per output per
        // entity.
        let computed_outputs: HashMap<Entity, HashMap<String, f32>> = (
            &read_data.entities,
            &wiring_elements,
            read_data.physics_states.maybe(),
            read_data.pos.maybe(),
        )
            .join()
            .map(|(entity, wiring_element, physics_state, pos)| {
                (
                    entity,
                    wiring_element
                        .outputs
                        .iter()
                        .map(|(key, output_formula)| {
                            (
                                // Output name/key
                                key.to_string(),
                                // Output value
                                output_formula.compute_output(
                                    &wiring_element.inputs,
                                    physics_state,
                                    &read_data.entities_died_last_tick.0,
                                    pos,
                                ),
                            )
                        })
                        .collect::<HashMap<_, _>>(),
                )
            })
            .collect();

        // Pass new outputs as inputs for the next tick to the proper elements in the
        // circuit.
        (read_data.circuits)
            .join()
            .flat_map(|circuit| circuit.wires.iter())
            .for_each(|wire| {
                // The current output values becomes input values for the next tick
                let input_value = computed_outputs
                    .get(&wire.input.entity)
                    .and_then(|e| e.get(&wire.input.name))
                    .unwrap_or(&0.0);

                // Push the current output value into the inputs for the proper element to be
                // used next tick.
                if let Some(wiring_element) = wiring_elements.get_mut(wire.output.entity) {
                    wiring_element
                        .inputs
                        .insert(wire.output.name.clone(), *input_value);
                }
            });

        // Use inputs to dispatch actions and apply effects
        (
            &read_data.entities,
            &mut wiring_elements,
            read_data.physics_states.maybe(),
            (&mut light_emitters).maybe(),
            read_data.pos.maybe(),
        )
            .lend_join()
            .for_each(
                |(entity, wiring_element, physics_state, mut light_emitter, pos)| {
                    wiring_element
                        .actions
                        .iter()
                        .filter(|wiring_action| {
                            // Filter out any wiring actions with a total output less than the
                            // threshold
                            wiring_action.formula.compute_output(
                                &wiring_element.inputs,
                                physics_state,
                                &read_data.entities_died_last_tick.0,
                                pos,
                            ) >= wiring_action.threshold
                        })
                        .for_each(|wiring_action| {
                            // Apply world effects of each wiring action
                            wiring_action.apply_effects(
                                entity,
                                &wiring_element.inputs,
                                physics_state,
                                &read_data.entities_died_last_tick.0,
                                &mut emitters,
                                pos,
                                &mut block_change,
                                light_emitter.as_deref_mut(),
                            );
                        })
                },
            )
    }
}
