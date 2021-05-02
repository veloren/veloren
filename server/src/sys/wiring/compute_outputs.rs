use super::WiringData;
use crate::wiring::OutputFormula;
use common::comp::PhysicsState;
use hashbrown::HashMap;
use specs::{join::Join, Entity, ReadStorage};
use tracing::warn;

pub fn compute_outputs(system_data: &WiringData) -> HashMap<Entity, HashMap<String, f32>> {
    let WiringData {
        entities,
        wiring_elements,
        physics_states,
        ..
    } = system_data;
    (&*entities, wiring_elements)
        .join()
        .map(|(entity, wiring_element)| {
            (
                entity,
                wiring_element
                    .outputs
                    .iter()
                    .map(
                        |(key, output_formula)| {
                            compute_output_with_key(
                                key,
                                output_formula,
                                &wiring_element.inputs,
                                entity,
                                physics_states,
                            )
                        }, // (String, f32)
                    )
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect()
}

pub fn compute_output_with_key(
    // yes, this function is defined only to make one place
    // look a bit nicer
    // Don't discuss.
    key: &str,
    output_formula: &OutputFormula,
    inputs: &HashMap<String, f32>,
    entity: Entity,
    physics_states: &ReadStorage<PhysicsState>,
) -> (String, f32) {
    (
        key.to_string(),
        compute_output(output_formula, inputs, entity, physics_states),
    )
}

pub fn compute_output(
    output_formula: &OutputFormula,
    inputs: &HashMap<String, f32>,
    entity: Entity,
    physics_states: &ReadStorage<PhysicsState>,
) -> f32 {
    match output_formula {
        OutputFormula::Constant { value } => *value,
        OutputFormula::Input { name } => *inputs.get(name).unwrap_or(&0.0),
        OutputFormula::Logic(_logic) => {
            warn!("Not implemented OutputFormula::Logic");
            0.0
        },
        OutputFormula::SineWave { .. } => {
            warn!("Not implemented OutputFormula::SineWave");
            0.0
        },
        OutputFormula::OnCollide { value } => {
            output_formula_on_collide(value, entity, physics_states)
        },
        OutputFormula::OnInteract { .. } => {
            warn!("Not implemented OutputFormula::OnInteract");
            0.0
        },
    }
}

fn output_formula_on_collide(
    value: &f32,
    entity: Entity,
    physics_states: &ReadStorage<PhysicsState>,
) -> f32 {
    if let Some(ps) = physics_states.get(entity) {
        if !ps.touch_entities.is_empty() {
            return *value;
        }
    }
    0.0
}
