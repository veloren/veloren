use super::WiringData;
use crate::wiring::{Logic, LogicKind, OutputFormula};
use common::{
    comp::{PhysicsState, Pos},
    resources::EntitiesDiedLastTick,
};
use hashbrown::HashMap;
use rand_distr::num_traits::ToPrimitive;
use specs::{join::Join, Entity, Read};
use tracing::warn;

pub fn compute_outputs(system_data: &WiringData) -> HashMap<Entity, HashMap<String, f32>> {
    let WiringData {
        entities,
        wiring_elements,
        physics_states,
        entities_died_last_tick,
        pos,
        ..
    } = system_data;
    (
        &*entities,
        wiring_elements,
        physics_states.maybe(),
        pos.maybe(),
    )
        .join()
        .map(|(entity, wiring_element, physics_state, pos)| {
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
                                physics_state,
                                entities_died_last_tick,
                                pos,
                            )
                        }, // (String, f32)
                    )
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn compute_output_with_key(
    // yes, this function is defined only to make one place
    // look a bit nicer
    // Don't discuss.
    key: &str,
    output_formula: &OutputFormula,
    inputs: &HashMap<String, f32>,
    physics_state: Option<&PhysicsState>,
    entities_died_last_tick: &Read<EntitiesDiedLastTick>,
    pos: Option<&Pos>,
) -> (String, f32) {
    (
        key.to_string(),
        compute_output(
            output_formula,
            inputs,
            physics_state,
            entities_died_last_tick,
            pos,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn compute_output(
    output_formula: &OutputFormula,
    inputs: &HashMap<String, f32>,
    physics_state: Option<&PhysicsState>,
    entities_died_last_tick: &Read<EntitiesDiedLastTick>,
    pos: Option<&Pos>,
) -> f32 {
    match output_formula {
        OutputFormula::Constant { value } => *value,
        OutputFormula::Input { name } => *inputs.get(name).unwrap_or(&0.0),
        OutputFormula::Logic(logic) => {
            output_formula_logic(logic, inputs, physics_state, entities_died_last_tick, pos)
        },
        OutputFormula::SineWave { .. } => {
            warn!("Not implemented OutputFormula::SineWave");
            0.0
        },
        OutputFormula::OnCollide { value } => output_formula_on_collide(value, physics_state),
        OutputFormula::OnInteract { .. } => {
            warn!("Not implemented OutputFormula::OnInteract");
            0.0
        },
        OutputFormula::OnDeath { value, radius } => {
            output_formula_on_death(value, radius, entities_died_last_tick, pos)
        },
    }
}

fn output_formula_on_collide(value: &f32, physics_state: Option<&PhysicsState>) -> f32 {
    if let Some(ps) = physics_state {
        if !ps.touch_entities.is_empty() {
            return *value;
        }
    }
    0.0
}

fn output_formula_on_death(
    value: &f32,
    radius: &f32,
    entities_died_last_tick: &Read<EntitiesDiedLastTick>,
    pos: Option<&Pos>,
) -> f32 {
    if let Some(pos_of_entity) = pos {
        return *value
            * entities_died_last_tick
                .0
                .iter()
                .filter(|(_, dead_pos)| pos_of_entity.0.distance(dead_pos.0) <= *radius)
                .count()
                .to_f32()
                .unwrap_or(0.0);
    }
    0.0
}

#[allow(clippy::too_many_arguments)]
fn output_formula_logic(
    logic: &Logic,
    inputs: &HashMap<String, f32>,
    physics_state: Option<&PhysicsState>,
    entities_died_last_tick: &Read<EntitiesDiedLastTick>,
    pos: Option<&Pos>,
) -> f32 {
    let left = compute_output(
        &logic.left,
        inputs,
        physics_state,
        entities_died_last_tick,
        pos,
    );
    let right = compute_output(
        &logic.right,
        inputs,
        physics_state,
        entities_died_last_tick,
        pos,
    );
    match logic.kind {
        LogicKind::Max => f32::max(left, right),
        LogicKind::Min => f32::min(left, right),
        LogicKind::Sub => left - right,
        LogicKind::Sum => left + right,
        LogicKind::Mul => left * right,
    }
}
