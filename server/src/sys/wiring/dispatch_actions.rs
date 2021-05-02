use super::{compute_outputs::compute_output, WiringData};
use crate::wiring::{OutputFormula, WiringActionEffect};
use common::{
    comp::{object, Body, LightEmitter, PhysicsState, ProjectileConstructor},
    event::{Emitter, ServerEvent},
    util::Dir,
};
use hashbrown::HashMap;
use specs::{join::Join, Entity, ReadStorage, WriteStorage};
use tracing::warn;
use vek::Rgb;

pub fn dispatch_actions(system_data: &mut WiringData) {
    let WiringData {
        entities,
        event_bus,
        wiring_elements,
        light_emitters,
        physics_states,
        ..
    } = system_data;
    let mut server_emitter = event_bus.emitter();

    (&*entities, wiring_elements)
        .join()
        .for_each(|(entity, wiring_element)| {
            wiring_element
                .actions
                .iter()
                .filter(|wiring_action| {
                    compute_output(
                        &wiring_action.formula,
                        &wiring_element.inputs,
                        entity,
                        physics_states,
                    ) >= wiring_action.threshold
                })
                .for_each(|wiring_action| {
                    dispatch_action(
                        entity,
                        &wiring_element.inputs,
                        &wiring_action.effects,
                        &mut server_emitter,
                        light_emitters,
                        physics_states,
                    );
                })
        })
}

fn dispatch_action(
    entity: Entity,
    source: &HashMap<String, f32>,
    action_effects: &[WiringActionEffect],

    server_emitter: &mut Emitter<ServerEvent>,
    light_emitters: &mut WriteStorage<LightEmitter>,
    physics_states: &ReadStorage<PhysicsState>,
) {
    action_effects
        .iter()
        .for_each(|action_effect| match action_effect {
            WiringActionEffect::SetBlockCollidability { .. } => {
                warn!("Not implemented WiringActionEffect::SetBlockCollidability")
            },
            WiringActionEffect::SpawnProjectile { constr } => {
                dispatch_action_spawn_projectile(entity, constr, server_emitter)
            },
            WiringActionEffect::SetLight { r, g, b } => {
                dispatch_action_set_light(entity, source, r, g, b, light_emitters, physics_states)
            },
        });
}

fn dispatch_action_spawn_projectile(
    entity: Entity,
    constr: &ProjectileConstructor,
    server_emitter: &mut Emitter<ServerEvent>,
) {
    // Use match here if there will be more options
    // NOTE: constr in RFC is about Arrow projectile
    server_emitter.emit(ServerEvent::Shoot {
        entity,
        dir: Dir::forward(),
        body: Body::Object(object::Body::Arrow),
        projectile: constr.create_projectile(None, 1.0, 1.0),
        light: None,
        speed: 5.0,
        object: None,
    });
}

fn dispatch_action_set_light(
    entity: Entity,
    source: &HashMap<String, f32>,
    r: &OutputFormula,
    g: &OutputFormula,
    b: &OutputFormula,

    light_emitters: &mut WriteStorage<LightEmitter>,
    physics_states: &ReadStorage<PhysicsState>,
) {
    let mut light_emitter = light_emitters.get_mut(entity).unwrap();

    // TODO: make compute_output accept multiple formulas
    let computed_r = compute_output(r, source, entity, physics_states);
    let computed_g = compute_output(g, source, entity, physics_states);
    let computed_b = compute_output(b, source, entity, physics_states);

    light_emitter.col = Rgb::new(computed_r, computed_g, computed_b);
}
