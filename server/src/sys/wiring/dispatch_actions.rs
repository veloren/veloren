use std::ops::DerefMut;

use super::{compute_outputs::compute_output, WiringData};
use crate::wiring::{OutputFormula, WiringActionEffect};
use common::{
    comp::{object, Body, LightEmitter, PhysicsState, ProjectileConstructor},
    event::{Emitter, ServerEvent},
    util::Dir,
};
use hashbrown::HashMap;
use specs::{join::Join, Entity};
use tracing::warn;
use vek::Rgb;

pub fn dispatch_actions(system_data: &mut WiringData) {
    let WiringData {
        entities,
        event_bus,
        wiring_elements,
        physics_states,
        light_emitters,
        ..
    } = system_data;
    let mut server_emitter = event_bus.emitter();

    (
        &*entities,
        wiring_elements,
        physics_states.maybe(),
        light_emitters.maybe(),
    )
        .join()
        .for_each(
            |(entity, wiring_element, physics_state, mut light_emitter)| {
                wiring_element
                    .actions
                    .iter()
                    .filter(|wiring_action| {
                        compute_output(
                            &wiring_action.formula,
                            &wiring_element.inputs,
                            physics_state,
                        ) >= wiring_action.threshold
                    })
                    .for_each(|wiring_action| {
                        dispatch_action(
                            entity,
                            &wiring_element.inputs,
                            &wiring_action.effects,
                            &mut server_emitter,
                            &mut light_emitter,
                            physics_state,
                        );
                    })
            },
        )
}

fn dispatch_action(
    entity: Entity,
    inputs: &HashMap<String, f32>,
    action_effects: &[WiringActionEffect],

    server_emitter: &mut Emitter<ServerEvent>,

    light_emitter: &mut Option<impl DerefMut<Target = LightEmitter>>,
    physics_state: Option<&PhysicsState>,
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
            WiringActionEffect::SetLight { r, g, b } => dispatch_action_set_light(
                inputs,
                r,
                g,
                b,
                &mut light_emitter.as_deref_mut(),
                physics_state,
            ),
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
    inputs: &HashMap<String, f32>,
    r: &OutputFormula,
    g: &OutputFormula,
    b: &OutputFormula,

    light_emitter: &mut Option<&mut LightEmitter>,

    physics_state: Option<&PhysicsState>,
) {
    if let Some(light_emitter) = light_emitter {
        // TODO: make compute_output accept multiple formulas

        let computed_r = compute_output(r, inputs, physics_state);
        let computed_g = compute_output(g, inputs, physics_state);
        let computed_b = compute_output(b, inputs, physics_state);

        light_emitter.col = Rgb::new(computed_r, computed_g, computed_b);
    }
}
