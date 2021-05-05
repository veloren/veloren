use crate::wiring::{Circuit, WiringElement};
use common::{
    comp::{LightEmitter, PhysicsState, Pos},
    event::{EventBus, ServerEvent},
    resources::EntitiesDiedLastTick,
};
use common_ecs::{Job, Origin, Phase, System};
use hashbrown::HashMap;
use specs::{
    join::Join, shred::ResourceId, Entities, Entity, Read, ReadStorage, SystemData, World,
    WriteStorage,
};
mod compute_outputs;
use compute_outputs::compute_outputs;
mod dispatch_actions;
use dispatch_actions::dispatch_actions;

#[derive(SystemData)]
pub struct WiringData<'a> {
    pub circuits: ReadStorage<'a, Circuit>,
    pub wiring_elements: WriteStorage<'a, WiringElement>,

    pub entities: Entities<'a>,

    pub light_emitters: WriteStorage<'a, LightEmitter>, // maybe
    pub physics_states: ReadStorage<'a, PhysicsState>,  // maybe
    pub pos: ReadStorage<'a, Pos>,

    pub event_bus: Read<'a, EventBus<ServerEvent>>,
    pub entities_died_last_tick: Read<'a, EntitiesDiedLastTick>,
}

/// This system is responsible for handling wiring (signals and wiring systems)
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = WiringData<'a>;

    const NAME: &'static str = "wiring";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, mut system_data: Self::SystemData) {
        // Calculate new outputs using inputs (those inputs are calculated and populated
        // in previous tick) Take inputs and wiring_element.outputs and with
        // that compute new outputs
        let computed_outputs = compute_outputs(&system_data);
        // Pass new outputs as inputs for the next tick
        dispatch_circuit_transport(&computed_outputs, &mut system_data);
        // Using inputs dispatch actions
        dispatch_actions(&mut system_data);
    }
}

fn dispatch_circuit_transport<'a>(
    computed_outputs: &HashMap<Entity, HashMap<String, f32>>,
    system_data: &mut WiringData<'a>,
) {
    let WiringData {
        circuits,
        wiring_elements,
        ..
    } = system_data;

    (circuits)
        .join()
        .map(|circuit| circuit.wires.iter())
        .flatten()
        .for_each(|wire| {
            let input_value = computed_outputs
                .get(&wire.input_entity)
                .and_then(|e| e.get(&wire.input_field))
                .unwrap_or(&0.0);

            if let Some(wiring_element) = wiring_elements.get_mut(wire.output_entity) {
                wiring_element
                    .inputs
                    .insert(wire.output_field.clone(), *input_value);
            }
        });
}
