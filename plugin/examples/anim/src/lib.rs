mod bindings;

use bindings::{
    exports::veloren::plugin::{
        animation::{self, GuestBody},
        events::Guest,
    },
    veloren::plugin::{
        actions,
        types::{self, Dependency, GameMode, Quaternion, Skeleton, Transform, Vec3},
    },
};
use core::f32::consts::PI;

#[derive(Default)]
pub struct Body {
    body: types::BodyIndex,
}

#[repr(u32)]
enum BodyType {
    None,
    Lizard,
}

impl From<i32> for BodyType {
    fn from(a: i32) -> Self {
        if a == BodyType::Lizard as i32 {
            BodyType::Lizard
        } else {
            BodyType::None
        }
    }
}

fn identity() -> Quaternion { vek::Quaternion::identity().into_vec4().into_tuple() }
fn one() -> Vec3 { vek::Vec3::one().into_tuple() }

fn static_limb(position: (f32, f32, f32)) -> Transform {
    Transform {
        position,
        orientation: identity(),
        scale: one(),
    }
}

fn rotate_z(position: (f32, f32, f32), rot_z: f32) -> Transform {
    Transform {
        position,
        orientation: (vek::Quaternion::rotation_z(rot_z))
            .into_vec4()
            .into_tuple(),
        scale: one(),
    }
}

fn rotate_x(position: (f32, f32, f32), rot_x: f32) -> Transform {
    Transform {
        position,
        orientation: (vek::Quaternion::rotation_x(rot_x))
            .into_vec4()
            .into_tuple(),
        scale: one(),
    }
}

impl GuestBody for Body {
    fn new(species: types::BodyIndex) -> Body { Body { body: species } }

    fn update_skeleton(&self, dependency: Dependency, anim_time: f32) -> Skeleton {
        let mut result = Skeleton::new();

        match (BodyType::from(self.body), dependency.state) {
            (BodyType::Lizard, types::CharacterState::Run) => {
                let fast = (anim_time * 10.0 + PI).sin();
                // head
                result.push(rotate_z((0.0, 2.0, 9.0), fast * -0.3));
                // chest
                result.push(static_limb((0.0, 0.0, 5.0)));
                //arm
                result.push(rotate_x((-2.0, -3.0, 7.0), fast * 0.6));
                result.push(rotate_x((2.0, -3.0, 7.0), -fast * 0.6));
                // leg
                result.push(rotate_x((-1.5, -1.0 + fast, 3.5), fast * -0.3));
                result.push(rotate_x((1.5, -1.0 - fast, 3.5), -fast * -0.3));
                // tail
                result.push(rotate_z((0.0, -1.0, 3.0), fast * -0.3));
            },
            (BodyType::Lizard, _) => {
                let slow = (anim_time * 3.5 + PI).sin();
                // head
                result.push(rotate_z((0.0, 2.0, 9.0), slow * 0.1));
                // chest
                result.push(static_limb((0.0, 0.0, 5.0)));
                //arm
                result.push(rotate_x((-2.0, -3.0, 7.0), slow * 0.1));
                result.push(rotate_x((2.0, -3.0, 7.0), -slow * 0.1));
                // leg
                result.push(rotate_x((-1.5, -1.0, 3.0), slow * -0.01));
                result.push(rotate_x((1.5, -1.0, 3.0), -slow * -0.01));
                // tail
                result.push(rotate_z((0.0, -1.0, 3.0), slow * -0.01));
            },
            _ => {},
        }

        result
    }
}

#[derive(Default)]
struct Component {}

impl Guest for Component {
    fn load(_mode: GameMode) { actions::register_animation("lizard", BodyType::Lizard as i32); }
}

impl animation::Guest for Component {
    type Body = Body;
}

bindings::export!(Component with_types_in bindings);
