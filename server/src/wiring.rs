use common::{
    comp::{item::tool, object, Body, LightEmitter, PhysicsState, Pos, ProjectileConstructor},
    event::{EmitExt, ShootEvent},
    terrain::{Block, TerrainChunkSize},
    util::Dir,
    vol::RectVolSize,
};
use common_state::BlockChange;
use hashbrown::HashMap;
use specs::{Component, DenseVecStorage, Entity};
use tracing::warn;
use vek::{num_traits::ToPrimitive, Rgb, Vec3};

/// Represents a logical operation based on a `left` and `right` input. The
/// available kinds of logical operations are enumerated by `LogicKind`.
pub struct Logic {
    pub kind: LogicKind,
    pub left: OutputFormula,
    pub right: OutputFormula,
}

/// The basic element of the wiring system. Inputs are dynamically added based
/// on the outputs of other elements. Actions specify what to output or what
/// inputs to read as well as what effects the values should have on the world
/// state (eg. emit a projectile).
pub struct WiringElement {
    pub inputs: HashMap<String, f32>,
    pub outputs: HashMap<String, OutputFormula>,
    pub actions: Vec<WiringAction>,
}

/// Connects input to output elements. Required for elements to receive outputs
/// from the proper inputs.
pub struct Circuit {
    pub wires: Vec<Wire>,
}

impl Circuit {
    pub fn new(wires: Vec<Wire>) -> Self { Self { wires } }
}

/// Represents an output for a `WiringAction`. The total output can be constant,
/// directly from an input, based on collision state, or based on custom logic.
pub enum OutputFormula {
    /// Returns a constant value
    Constant { value: f32 },
    /// Retrieves the value from a string identified input. A wiring element can
    /// have multiple inputs.
    Input { name: String },
    /// Performs a logic operation on the `left` and `right` values of the
    /// provided `Logic`. The operation is specified by the `LogicKind`.
    /// Operations include `Min`, `Max`, `Sub`, `Sum`, and `Mul`.
    Logic(Box<Logic>),
    /// Returns `value` if the wiring element is in contact with another entity
    /// with collision.
    OnCollide { value: f32 },
    /// Returns `value` if an entity died in the last tick within `radius` of
    /// the wiring element.
    OnDeath { value: f32, radius: f32 },

    // TODO: The following `OutputFormula`s are unimplemented!!!!
    /// Returns an oscillating value based on the sine wave with `amplitude` and
    /// `frequency`.
    SineWave { amplitude: f32, frequency: f32 },
    /// Returns `value` when the wiring element in interacted with.
    OnInteract { value: f32 },
}

impl OutputFormula {
    /// Computes the output of an `OutputFormula` as an `f32` based on the
    /// inputs and world state. Currently that world state only includes
    /// physics state, position, and the list of entities that died in the
    /// last tick.
    pub fn compute_output(
        &self,
        inputs: &HashMap<String, f32>,
        physics_state: Option<&PhysicsState>,
        entities_died_last_tick: &Vec<(Entity, Pos)>,
        pos: Option<&Pos>,
    ) -> f32 {
        match self {
            OutputFormula::Constant { value } => *value,
            OutputFormula::Input { name } => *inputs.get(name).unwrap_or(&0.0),
            OutputFormula::Logic(logic) => {
                let left =
                    &logic
                        .left
                        .compute_output(inputs, physics_state, entities_died_last_tick, pos);
                let right = &logic.right.compute_output(
                    inputs,
                    physics_state,
                    entities_died_last_tick,
                    pos,
                );
                match logic.kind {
                    LogicKind::Max => f32::max(*left, *right),
                    LogicKind::Min => f32::min(*left, *right),
                    LogicKind::Sub => left - right,
                    LogicKind::Sum => left + right,
                    LogicKind::Mul => left * right,
                }
            },
            OutputFormula::OnCollide { value } => physics_state.map_or(0.0, |ps| {
                if ps.touch_entities.is_empty() {
                    0.0
                } else {
                    *value
                }
            }),
            OutputFormula::SineWave { .. } => {
                warn!("Not implemented OutputFormula::SineWave");
                0.0
            },
            OutputFormula::OnInteract { .. } => {
                warn!("Not implemented OutputFormula::OnInteract");
                0.0
            },
            OutputFormula::OnDeath { value, radius } => pos.map_or(0.0, |e_pos| {
                *value
                    * entities_died_last_tick
                        .iter()
                        .filter(|(_, dead_pos)| e_pos.0.distance(dead_pos.0) <= *radius)
                        .count()
                        .to_f32()
                        .unwrap_or(0.0)
            }),
        }
    }
}

/// Logical operations applied to two floats.
pub enum LogicKind {
    /// Returns the minimum of `left` and `right`. Acts like And.
    Min,
    /// Returns the maximum of `left` and `right`. Acts like Or.
    Max,
    /// Returns `left` minus `right`. Acts like Not, depending on referance
    /// values.
    Sub,
    /// Returns `left` plus `right`.
    Sum,
    /// Returns `left` times `right`.
    Mul,
}

/// Determines what kind of output an element produces (or input is read) based
/// on the `formula`. The `threshold` is the minimum computed output for effects
/// to take place. Effects refer to effects in the game world such as emitting
/// light.
pub struct WiringAction {
    pub formula: OutputFormula,
    pub threshold: f32,
    pub effects: Vec<WiringActionEffect>,
}

impl WiringAction {
    /// Applies all effects on the world (such as turning on a light etc.) if
    /// the output of the `formula` is greater than `threshold`.
    pub fn apply_effects(
        &self,
        entity: Entity,
        inputs: &HashMap<String, f32>,
        physics_state: Option<&PhysicsState>,
        entities_died_last_tick: &Vec<(Entity, Pos)>,
        emitters: &mut impl EmitExt<ShootEvent>,
        pos: Option<&Pos>,
        block_change: &mut BlockChange,
        mut light_emitter: Option<&mut LightEmitter>,
    ) {
        self.effects
            .iter()
            .for_each(|action_effect| match action_effect {
                WiringActionEffect::SetBlock { coords, block } => {
                    let chunk_origin = pos.map_or(Vec3::zero(), |opos| {
                        opos.0
                            .xy()
                            .as_::<i32>()
                            .map2(TerrainChunkSize::RECT_SIZE.as_::<i32>(), |a, b| (a / b) * b)
                            .with_z(0)
                    });
                    let offset_pos = chunk_origin + coords;
                    block_change.set(offset_pos, *block);
                },
                WiringActionEffect::SpawnProjectile { constr } => {
                    if let Some(&pos) = pos {
                        emitters.emit(ShootEvent {
                            entity,
                            pos,
                            dir: Dir::forward(),
                            body: Body::Object(object::Body::Arrow),
                            projectile: constr.create_projectile(
                                None,
                                1.0,
                                tool::Stats::one(),
                                None,
                            ),
                            light: None,
                            speed: 5.0,
                            object: None,
                        });
                    }
                },
                WiringActionEffect::SetLight { r, g, b } => {
                    if let Some(light_emitter) = &mut light_emitter {
                        let computed_r =
                            r.compute_output(inputs, physics_state, entities_died_last_tick, pos);
                        let computed_g =
                            g.compute_output(inputs, physics_state, entities_died_last_tick, pos);
                        let computed_b =
                            b.compute_output(inputs, physics_state, entities_died_last_tick, pos);

                        light_emitter.col = Rgb::new(computed_r, computed_g, computed_b);
                    }
                },
            });
    }
}

/// Effects of a circuit in the game world.
pub enum WiringActionEffect {
    /// Spawn a projectile.
    SpawnProjectile { constr: ProjectileConstructor },
    /// Set a terrain block at the provided coordinates.
    SetBlock { coords: Vec3<i32>, block: Block },
    /// Emit light with the given RGB values.
    SetLight {
        r: OutputFormula,
        g: OutputFormula,
        b: OutputFormula,
    },
}

/// Holds an input and output node.
pub struct Wire {
    pub input: WireNode,
    pub output: WireNode,
}

/// Represents a node in the circuit. Each node is an entity with a name.
pub struct WireNode {
    pub entity: Entity,
    pub name: String,
}

impl WireNode {
    pub fn new(entity: Entity, name: String) -> Self { Self { entity, name } }
}

impl Component for WiringElement {
    type Storage = DenseVecStorage<Self>;
}

impl Component for Circuit {
    type Storage = DenseVecStorage<Self>;
}
