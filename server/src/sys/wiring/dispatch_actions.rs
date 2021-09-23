use std::ops::DerefMut;

use super::{compute_outputs::compute_output, WiringData};
use crate::wiring::{OutputFormula, WiringActionEffect};
use common::{
    comp::{object, Body, LightEmitter, PhysicsState, Pos, ProjectileConstructor},
    event::{Emitter, ServerEvent},
    resources::EntitiesDiedLastTick,
    terrain::{block::Block, TerrainChunkSize},
    util::Dir,
    vol::RectVolSize,
};
use common_state::BlockChange;
use hashbrown::HashMap;
use specs::{join::Join, Entity, Read, Write};
use vek::{Rgb, Vec3};

pub fn dispatch_actions(system_data: &mut WiringData) {
    let WiringData {
        entities,
        event_bus,
        wiring_elements,
        physics_states,
        light_emitters,
        entities_died_last_tick,
        block_change,
        pos,
        ..
    } = system_data;
    let mut server_emitter = event_bus.emitter();

    (
        &*entities,
        wiring_elements,
        physics_states.maybe(),
        light_emitters.maybe(),
        pos.maybe(),
    )
        .join()
        .for_each(
            |(entity, wiring_element, physics_state, mut light_emitter, pos)| {
                wiring_element
                    .actions
                    .iter()
                    .filter(|wiring_action| {
                        compute_output(
                            &wiring_action.formula,
                            &wiring_element.inputs,
                            physics_state,
                            entities_died_last_tick,
                            pos,
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
                            entities_died_last_tick,
                            block_change,
                            pos,
                        );
                    })
            },
        )
}

#[allow(clippy::too_many_arguments)]
fn dispatch_action(
    entity: Entity,
    inputs: &HashMap<String, f32>,
    action_effects: &[WiringActionEffect],

    server_emitter: &mut Emitter<ServerEvent>,

    light_emitter: &mut Option<impl DerefMut<Target = LightEmitter>>,
    physics_state: Option<&PhysicsState>,
    entities_died_last_tick: &Read<EntitiesDiedLastTick>,
    block_change: &mut Write<BlockChange>,
    pos: Option<&Pos>,
) {
    action_effects
        .iter()
        .for_each(|action_effect| match action_effect {
            WiringActionEffect::SetBlock { coords, block } => {
                dispatch_action_set_block(*coords, *block, block_change, pos);
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
                entities_died_last_tick,
                pos,
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
        pos: Pos(Vec3::zero()),
        dir: Dir::forward(),
        body: Body::Object(object::Body::Arrow),
        projectile: constr.create_projectile(None, 0.0, 1.0, 1.0),
        light: None,
        speed: 5.0,
        object: None,
    });
}

#[allow(clippy::too_many_arguments)]
fn dispatch_action_set_light(
    inputs: &HashMap<String, f32>,
    r: &OutputFormula,
    g: &OutputFormula,
    b: &OutputFormula,

    light_emitter: &mut Option<&mut LightEmitter>,

    physics_state: Option<&PhysicsState>,
    entities_died_last_tick: &Read<EntitiesDiedLastTick>,
    pos: Option<&Pos>,
) {
    if let Some(light_emitter) = light_emitter {
        // TODO: make compute_output accept multiple formulas

        let computed_r = compute_output(r, inputs, physics_state, entities_died_last_tick, pos);
        let computed_g = compute_output(g, inputs, physics_state, entities_died_last_tick, pos);
        let computed_b = compute_output(b, inputs, physics_state, entities_died_last_tick, pos);

        light_emitter.col = Rgb::new(computed_r, computed_g, computed_b);
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_action_set_block(
    coord: vek::Vec3<i32>,
    block: Block,
    block_change: &mut Write<BlockChange>,
    pos: Option<&Pos>,
) {
    let chunk_origin = pos
        .map(|opos| {
            opos.0
                .xy()
                .as_::<i32>()
                .map2(TerrainChunkSize::RECT_SIZE.as_::<i32>(), |a, b| (a / b) * b)
                .with_z(0)
        })
        .unwrap_or_else(vek::Vec3::zero);
    let offset_pos = chunk_origin + coord;
    block_change.set(offset_pos, block);
}
