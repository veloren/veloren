pub mod combomelee;
pub mod dash;
pub mod feed;
pub mod hoof;
pub mod idle;
pub mod jump;
pub mod leapmelee;
pub mod run;
pub mod shockwave;
pub mod stunned;

// Reexports
pub use self::{
    combomelee::ComboAnimation, dash::DashAnimation, feed::FeedAnimation, hoof::HoofAnimation,
    idle::IdleAnimation, jump::JumpAnimation, leapmelee::LeapMeleeAnimation, run::RunAnimation,
    shockwave::ShockwaveAnimation, stunned::StunnedAnimation,
};

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};
use common::{
    comp::{self},
    states::utils::StageSection,
};
use core::convert::TryFrom;

pub type Body = comp::quadruped_medium::Body;

skeleton_impls!(struct QuadrupedMediumSkeleton {
    + head,
    + neck,
    + jaw,
    + tail,
    + torso_front,
    + torso_back,
    + ears,
    + leg_fl,
    + leg_fr,
    + leg_bl,
    + leg_br,
    + foot_fl,
    + foot_fr,
    + foot_bl,
    + foot_br,
    mount,
});

impl Skeleton for QuadrupedMediumSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 15;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"quadruped_medium_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 11.0);

        let torso_front_mat = base_mat * Mat4::<f32>::from(self.torso_front);
        let torso_back_mat = torso_front_mat * Mat4::<f32>::from(self.torso_back);
        let neck_mat = torso_front_mat * Mat4::<f32>::from(self.neck);
        let leg_fl_mat = torso_front_mat * Mat4::<f32>::from(self.leg_fl);
        let leg_fr_mat = torso_front_mat * Mat4::<f32>::from(self.leg_fr);
        let leg_bl_mat = torso_back_mat * Mat4::<f32>::from(self.leg_bl);
        let leg_br_mat = torso_back_mat * Mat4::<f32>::from(self.leg_br);
        let head_mat = neck_mat * Mat4::<f32>::from(self.head);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(neck_mat),
            make_bone(head_mat * Mat4::<f32>::from(self.jaw)),
            make_bone(torso_back_mat * Mat4::<f32>::from(self.tail)),
            make_bone(torso_front_mat),
            make_bone(torso_back_mat),
            make_bone(head_mat * Mat4::<f32>::from(self.ears)),
            make_bone(leg_fl_mat),
            make_bone(leg_fr_mat),
            make_bone(leg_bl_mat),
            make_bone(leg_br_mat),
            make_bone(leg_fl_mat * Mat4::<f32>::from(self.foot_fl)),
            make_bone(leg_fr_mat * Mat4::<f32>::from(self.foot_fr)),
            make_bone(leg_bl_mat * Mat4::<f32>::from(self.foot_bl)),
            make_bone(leg_br_mat * Mat4::<f32>::from(self.foot_br)),
        ];

        use comp::quadruped_medium::Species::*;
        let (mount_bone_mat, mount_bone_ori) = match (body.species, body.body_type) {
            (Mammoth, _) => (
                head_mat,
                self.torso_front.orientation * self.neck.orientation * self.head.orientation,
            ),
            _ => (torso_front_mat, self.torso_front.orientation),
        };
        // Offset from the mounted bone's origin.
        // Note: This could be its own bone if we need to animate it independently.
        let mount_position = (mount_bone_mat * Vec4::from_point(mount_point(&body)))
            .homogenized()
            .xyz();
        // NOTE: We apply the ori from base_mat externally so we don't need to worry
        // about it here for now.
        let mount_orientation = mount_bone_ori;

        Offsets {
            lantern: None,
            viewpoint: match body.species {
                Akhlut | Catoblepas | Lion => {
                    Some((head_mat * Vec4::new(0.0, 8.0, 0.0, 1.0)).xyz())
                },
                Barghest | Saber => Some((head_mat * Vec4::new(0.0, 8.0, 3.0, 1.0)).xyz()),
                Cattle | Highland | Bonerattler | Ngoubou | Yak => {
                    Some((head_mat * Vec4::new(0.0, 6.0, -1.0, 1.0)).xyz())
                },
                Antelope | Deer | Donkey | Bear | Mouflon | Panda => {
                    Some((head_mat * Vec4::new(0.0, 3.0, 3.0, 1.0)).xyz())
                },
                Camel | Hirdrasil | Horse | Kelpie | Zebra => {
                    Some((head_mat * Vec4::new(0.0, 2.0, 5.0, 1.0)).xyz())
                },
                Darkhound | Llama | Snowleopard | Tiger | Wolf | ClaySteed => {
                    Some((head_mat * Vec4::new(0.0, 4.0, 1.0, 1.0)).xyz())
                },
                Dreadhorn | Mammoth | Moose | Tarasque => {
                    Some((head_mat * Vec4::new(0.0, 13.0, -3.0, 1.0)).xyz())
                },
                Frostfang => Some((head_mat * Vec4::new(0.0, 5.0, 3.0, 1.0)).xyz()),
                Grolgar | Roshwalr => Some((head_mat * Vec4::new(0.0, 8.0, 6.0, 1.0)).xyz()),
                _ => Some((head_mat * Vec4::new(0.0, 2.0, 0.0, 1.0)).xyz()),
            },
            mount_bone: Transform {
                position: mount_position,
                orientation: mount_orientation,
                scale: Vec3::one(),
            },
            primary_trail_mat: None,
            secondary_trail_mat: None,
        }
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    neck: (f32, f32),
    jaw: (f32, f32),
    tail: (f32, f32),
    torso_back: (f32, f32),
    torso_front: (f32, f32),
    ears: (f32, f32),
    leg_f: (f32, f32, f32),
    leg_b: (f32, f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    scaler: f32,
    startangle: f32,
    tempo: f32,
    spring: f32,
    feed: (bool, f32),
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::QuadrupedMedium(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            neck: (0.0, 0.0),
            jaw: (0.0, 0.0),
            tail: (0.0, 0.0),
            torso_back: (0.0, 0.0),
            torso_front: (0.0, 0.0),
            ears: (0.0, 0.0),
            leg_f: (0.0, 0.0, 0.0),
            leg_b: (0.0, 0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            scaler: 0.0,
            startangle: 0.0,
            tempo: 0.0,
            spring: 0.0,
            feed: (false, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::quadruped_medium::{BodyType::*, Species::*};
        Self {
            head: match (body.species, body.body_type) {
                (Grolgar, _) => (0.0, -1.0),
                (Saber, _) => (5.0, -3.0),
                (Tuskram, _) => (0.0, 0.0),
                (Lion, Male) => (4.5, 2.0),
                (Lion, Female) => (2.5, -2.0),
                (Tarasque, _) => (-4.0, 3.5),
                (Tiger, _) => (2.0, 1.0),
                (Wolf, _) => (1.5, 3.0),
                (Frostfang, _) => (1.0, -2.0),
                (Mouflon, _) => (0.5, 1.5),
                (Catoblepas, _) => (-1.0, -6.5),
                (Bonerattler, _) => (0.0, 1.5),
                (Deer, Male) => (1.5, 3.5),
                (Deer, Female) => (1.5, 3.5),
                (Hirdrasil, _) => (0.0, 5.0),
                (Roshwalr, _) => (1.0, 0.5),
                (Donkey, _) => (4.5, -3.0),
                (Camel, _) => (-0.5, 5.0),
                (Zebra, _) => (3.0, -2.0),
                (Antelope, _) => (1.5, 2.5),
                (Kelpie, _) => (4.0, -1.0),
                (Horse, _) => (4.5, 2.5),
                (Barghest, _) => (0.5, -2.5),
                (Cattle, Male) => (2.0, 3.5),
                (Cattle, Female) => (2.5, 4.0),
                (Darkhound, _) => (3.0, -1.0),
                (Highland, _) => (2.5, 5.0),
                (Yak, _) => (2.5, 5.0),
                (Panda, _) => (0.0, 0.5),
                (Bear, _) => (0.5, 1.5),
                (Dreadhorn, _) => (-2.5, 7.0),
                (Moose, Male) => (-0.5, 5.0),
                (Moose, Female) => (3.5, 0.5),
                (Snowleopard, _) => (1.5, 0.5),
                (Mammoth, _) => (0.5, -1.5),
                (Ngoubou, _) => (0.5, -2.5),
                (Llama, _) => (0.5, 10.0),
                (Alpaca, _) => (0.5, 7.5),
                (Akhlut, _) => (1.0, 3.5),
                (Bristleback, _) => (-3.0, -2.0),
                (ClaySteed, _) => (-0.5, 6.0),
            },
            neck: match (body.species, body.body_type) {
                (Grolgar, _) => (1.0, -1.0),
                (Saber, _) => (-3.5, -2.0),
                (Tuskram, _) => (-1.0, 1.0),
                (Lion, Male) => (-1.5, 1.0),
                (Lion, Female) => (-2.0, 8.5),
                (Tarasque, _) => (-1.5, -4.0),
                (Tiger, _) => (0.0, 0.0),
                (Wolf, _) => (-4.5, 2.0),
                (Frostfang, _) => (0.5, 1.5),
                (Mouflon, _) => (-1.0, 1.0),
                (Catoblepas, _) => (19.5, -2.0),
                (Bonerattler, _) => (7.0, -0.5),
                (Deer, _) => (-2.5, 1.0),
                (Hirdrasil, _) => (-1.0, 0.5),
                (Roshwalr, _) => (0.0, 1.0),
                (Donkey, _) => (1.0, 3.5),
                (Camel, _) => (3.5, -1.5),
                (Zebra, _) => (1.0, 3.5),
                (Antelope, _) => (0.5, 2.5),
                (Kelpie, _) => (2.0, 1.0),
                (Horse, _) => (-2.5, -1.5),
                (Barghest, _) => (0.5, -0.5),
                (Cattle, Male) => (0.0, 0.0),
                (Cattle, Female) => (0.0, 0.0),
                (Darkhound, _) => (1.0, 1.5),
                (Highland, _) => (0.0, 1.5),
                (Yak, _) => (0.0, 0.0),
                (Panda, _) => (0.5, 0.0),
                (Bear, _) => (0.5, 0.0),
                (Dreadhorn, _) => (0.5, 0.0),
                (Moose, _) => (-0.5, 0.5),
                (Snowleopard, _) => (0.0, 1.5),
                (Mammoth, _) => (0.5, -0.5),
                (Ngoubou, _) => (2.0, 1.0),
                (Llama, _) => (2.5, 4.5),
                (Alpaca, _) => (-1.5, 3.0),
                (Akhlut, _) => (8.5, -1.0),
                (Bristleback, _) => (6.0, 2.5),
                (ClaySteed, _) => (1.5, 1.5),
            },
            jaw: match (body.species, body.body_type) {
                (Grolgar, _) => (7.0, 2.0),
                (Saber, _) => (2.5, -2.0),
                (Tuskram, _) => (4.0, -5.0),
                (Lion, Male) => (3.5, -4.0),
                (Lion, Female) => (3.5, -4.0),
                (Tarasque, _) => (9.0, -9.5),
                (Tiger, _) => (3.0, -3.5),
                (Wolf, _) => (5.0, -2.5),
                (Frostfang, _) => (4.0, -2.5),
                (Mouflon, _) => (6.0, 1.0),
                (Catoblepas, _) => (1.0, -3.5),
                (Bonerattler, _) => (3.0, -2.5),
                (Deer, _) => (3.5, 2.5),
                (Hirdrasil, _) => (2.5, 3.0),
                (Roshwalr, _) => (4.0, -1.0),
                (Donkey, _) => (1.0, 1.0),
                (Camel, _) => (2.0, 2.5),
                (Zebra, _) => (4.0, 0.0),
                (Antelope, _) => (3.0, 0.5),
                (Kelpie, _) => (1.0, 1.0),
                (Horse, _) => (4.0, 1.0),
                (Barghest, _) => (6.5, -3.0),
                (Cattle, Male) => (5.0, -5.5),
                (Cattle, Female) => (5.0, -5.0),
                (Darkhound, _) => (2.0, -2.0),
                (Highland, _) => (5.0, -6.0),
                (Yak, _) => (6.0, -8.0),
                (Panda, _) => (3.0, -3.0),
                (Bear, _) => (3.5, -2.0),
                (Dreadhorn, _) => (7.0, -5.0),
                (Moose, Male) => (10.0, -7.0),
                (Moose, Female) => (6.0, -2.5),
                (Snowleopard, _) => (3.0, -3.0),
                (Mammoth, _) => (9.5, -3.0),
                (Ngoubou, _) => (1.5, -4.0),
                (Llama, _) => (4.0, -1.0),
                (Alpaca, _) => (3.0, -2.5),
                (Akhlut, _) => (0.0, -4.5),
                (Bristleback, _) => (8.0, -6.0),
                (ClaySteed, _) => (4.0, -1.0),
            },
            tail: match (body.species, body.body_type) {
                (Grolgar, _) => (-11.5, -0.5),
                (Saber, _) => (-11.0, 0.0),
                (Tuskram, _) => (-8.0, 2.0),
                (Lion, Male) => (-11.0, 1.0),
                (Lion, Female) => (-11.0, 1.0),
                (Tarasque, _) => (-11.0, 0.0),
                (Tiger, _) => (-13.5, 3.0),
                (Wolf, _) => (-11.0, 0.0),
                (Frostfang, _) => (-7.0, -3.5),
                (Mouflon, _) => (-10.5, 3.0),
                (Catoblepas, _) => (-8.0, -2.0),
                (Bonerattler, _) => (-10.0, 1.5),
                (Deer, _) => (-8.5, 0.5),
                (Hirdrasil, _) => (-11.0, 2.0),
                (Roshwalr, _) => (-8.5, -1.0),
                (Donkey, _) => (-11.0, 1.5),
                (Camel, _) => (-14.0, -1.0),
                (Zebra, _) => (-10.0, 1.5),
                (Antelope, _) => (-10.0, 2.0),
                (Kelpie, _) => (-9.0, 3.0),
                (Horse, _) => (-9.0, 1.5),
                (Barghest, _) => (-7.0, -4.0),
                (Cattle, Male) => (-8.0, 3.5),
                (Cattle, Female) => (-8.0, 5.5),
                (Darkhound, _) => (-9.0, -3.0),
                (Highland, _) => (-9.0, 5.0),
                (Yak, _) => (-8.0, 2.5),
                (Panda, _) => (-9.5, 0.0),
                (Bear, _) => (-10.0, -0.5),
                (Dreadhorn, _) => (-5.5, 1.5),
                (Moose, _) => (-12.5, 3.5),
                (Snowleopard, _) => (-10.5, 3.0),
                (Mammoth, _) => (-13.0, -1.5),
                (Ngoubou, _) => (-12.0, 5.5),
                (Llama, _) => (-9.0, 6.0),
                (Alpaca, _) => (-8.5, 3.5),
                (Akhlut, _) => (-14.0, -2.0),
                (Bristleback, _) => (-7.0, -5.5),
                (ClaySteed, _) => (-11.0, 4.0),
            },
            torso_front: match (body.species, body.body_type) {
                (Grolgar, _) => (10.0, 13.0),
                (Saber, _) => (14.0, 13.0),
                (Tuskram, _) => (10.0, 16.0),
                (Lion, Male) => (10.0, 13.0),
                (Lion, Female) => (10.0, 13.5),
                (Tarasque, _) => (11.5, 17.5),
                (Tiger, _) => (10.0, 13.0),
                (Wolf, _) => (12.0, 13.0),
                (Frostfang, _) => (9.0, 11.5),
                (Mouflon, _) => (11.0, 14.0),
                (Catoblepas, _) => (7.5, 19.5),
                (Bonerattler, _) => (6.0, 11.0),
                (Deer, _) => (11.0, 13.5),
                (Hirdrasil, _) => (11.0, 14.5),
                (Roshwalr, _) => (6.0, 12.5),
                (Donkey, _) => (10.0, 15.5),
                (Camel, _) => (11.0, 22.5),
                (Zebra, _) => (10.0, 16.5),
                (Antelope, _) => (10.0, 14.0),
                (Kelpie, _) => (10.0, 16.0),
                (Horse, _) => (7.0, 16.0),
                (Barghest, _) => (11.5, 15.5),
                (Cattle, Male) => (7.0, 15.5),
                (Cattle, Female) => (7.0, 14.5),
                (Darkhound, _) => (7.0, 14.0),
                (Highland, _) => (7.0, 12.5),
                (Yak, _) => (7.0, 15.5),
                (Panda, _) => (7.0, 13.5),
                (Bear, _) => (7.0, 14.5),
                (Dreadhorn, _) => (1.5, 15.5),
                (Moose, _) => (1.5, 19.5),
                (Snowleopard, _) => (1.5, 13.0),
                (Mammoth, _) => (11.5, 20.5),
                (Ngoubou, _) => (9.5, 16.5),
                (Llama, _) => (7.0, 15.0),
                (Alpaca, _) => (7.0, 11.5),
                (Akhlut, _) => (5.5, 14.5),
                (Bristleback, _) => (1.5, 9.0),
                (ClaySteed, _) => (7.0, 15.0),
            },
            torso_back: match (body.species, body.body_type) {
                (Grolgar, _) => (-10.0, 1.5),
                (Saber, _) => (-13.5, 0.0),
                (Tuskram, _) => (-12.0, -2.5),
                (Lion, Male) => (-12.0, -0.5),
                (Lion, Female) => (-12.0, -0.5),
                (Tarasque, _) => (-14.0, -1.0),
                (Tiger, _) => (-13.0, -0.5),
                (Wolf, _) => (-12.5, 1.0),
                (Frostfang, _) => (-10.5, 0.0),
                (Mouflon, _) => (-8.5, -0.5),
                (Catoblepas, _) => (-8.5, -4.5),
                (Bonerattler, _) => (-5.0, 0.0),
                (Deer, _) => (-9.0, 0.5),
                (Hirdrasil, _) => (-9.0, -0.5),
                (Roshwalr, _) => (-9.0, -3.5),
                (Donkey, _) => (-6.0, -1.0),
                (Camel, _) => (-12.0, -0.5),
                (Zebra, _) => (-6.0, -1.0),
                (Antelope, _) => (-7.0, 0.0),
                (Kelpie, _) => (-8.0, -1.0),
                (Horse, _) => (-8.0, -1.5),
                (Barghest, _) => (-9.0, -1.5),
                (Cattle, Male) => (-8.0, -0.5),
                (Cattle, Female) => (-10.0, -2.0),
                (Darkhound, _) => (-12.0, 0.5),
                (Highland, _) => (-8.0, -0.5),
                (Yak, _) => (-8.0, -0.5),
                (Panda, _) => (-11.0, -0.5),
                (Bear, _) => (-11.0, -0.5),
                (Dreadhorn, _) => (-20.0, -1.0),
                (Moose, _) => (-10.0, -1.0),
                (Snowleopard, _) => (-11.0, 0.0),
                (Mammoth, _) => (-13.0, -2.5),
                (Ngoubou, _) => (-8.0, -2.0),
                (Llama, _) => (-8.0, 0.0),
                (Alpaca, _) => (-6.0, 0.0),
                (Akhlut, _) => (-7.0, 1.0),
                (Bristleback, _) => (-4.0, 2.0),
                (ClaySteed, _) => (-6.0, 0.0),
            },
            ears: match (body.species, body.body_type) {
                (Grolgar, _) => (5.0, 8.0),
                (Saber, _) => (3.0, 5.5),
                (Tuskram, _) => (0.0, 0.0),
                (Lion, Male) => (2.0, 3.5),
                (Lion, Female) => (2.0, 1.0),
                (Tarasque, _) => (12.0, -3.0),
                (Tiger, _) => (2.5, 4.0),
                (Wolf, _) => (3.0, 2.5),
                (Frostfang, _) => (2.0, 3.5),
                (Mouflon, _) => (2.5, 5.0),
                (Catoblepas, _) => (11.0, -3.0),
                (Bonerattler, _) => (2.0, 3.5),
                (Deer, _) => (2.5, 5.0),
                (Hirdrasil, _) => (2.5, 5.0),
                (Roshwalr, _) => (5.0, 8.0),
                (Donkey, _) => (-1.0, 8.0),
                (Camel, _) => (2.5, 5.0),
                (Zebra, _) => (0.0, 7.0),
                (Antelope, _) => (2.5, 5.0),
                (Kelpie, _) => (1.0, 7.5),
                (Horse, _) => (1.0, 7.0),
                (Barghest, _) => (12.0, -3.0),
                (Cattle, Male) => (2.0, -1.5),
                (Cattle, Female) => (2.0, -1.5),
                (Darkhound, _) => (1.0, 2.5),
                (Highland, _) => (2.0, -1.5),
                (Yak, _) => (3.0, -5.0),
                (Panda, _) => (1.0, 4.0),
                (Bear, _) => (1.0, 4.0),
                (Dreadhorn, _) => (1.5, 3.0),
                (Moose, Male) => (6.0, 1.0),
                (Moose, Female) => (2.0, 4.5),
                (Snowleopard, _) => (1.5, 3.0),
                (Mammoth, _) => (12.0, -3.0),
                (Ngoubou, _) => (12.0, -3.0),
                (Llama, _) => (1.0, 3.5),
                (Alpaca, _) => (1.0, 2.0),
                (Akhlut, _) => (12.0, -3.0),
                (Bristleback, _) => (6.0, 1.0),
                (ClaySteed, _) => (1.0, 3.5),
            },
            leg_f: match (body.species, body.body_type) {
                (Grolgar, _) => (7.5, -5.5, -1.0),
                (Saber, _) => (7.0, -4.0, -2.5),
                (Tuskram, _) => (8.5, -4.5, -2.0),
                (Lion, Male) => (6.5, -6.5, -1.5),
                (Lion, Female) => (6.5, -6.5, -1.5),
                (Tarasque, _) => (7.0, -8.0, -6.0),
                (Tiger, _) => (6.0, -6.0, -1.5),
                (Wolf, _) => (4.5, -6.5, -1.5),
                (Frostfang, _) => (5.5, -5.5, -2.0),
                (Mouflon, _) => (4.0, -5.0, -4.0),
                (Catoblepas, _) => (7.0, 2.0, -5.0),
                (Bonerattler, _) => (5.5, 5.0, -2.5),
                (Deer, _) => (3.5, -4.5, -3.5),
                (Hirdrasil, _) => (4.5, -5.0, -2.5),
                (Roshwalr, _) => (8.0, -2.5, -2.5),
                (Donkey, _) => (4.0, -3.5, -4.0),
                (Camel, _) => (4.5, -3.5, -5.5),
                (Zebra, _) => (4.0, -2.5, -4.5),
                (Antelope, _) => (4.0, -4.5, -2.5),
                (Kelpie, _) => (4.5, -3.5, -3.5),
                (Horse, _) => (4.5, -2.5, -3.0),
                (Barghest, _) => (9.5, 0.0, -2.5),
                (Cattle, Male) => (5.5, -2.0, -2.5),
                (Cattle, Female) => (5.5, -2.5, -1.0),
                (Darkhound, _) => (4.0, -6.5, -2.0),
                (Highland, _) => (5.5, -2.5, 0.0),
                (Yak, _) => (4.5, -2.0, -1.5),
                (Panda, _) => (7.5, -5.5, -2.0),
                (Bear, _) => (5.5, -4.5, -3.5),
                (Dreadhorn, _) => (8.5, -7.0, -0.5),
                (Moose, _) => (5.5, -4.0, 1.0),
                (Snowleopard, _) => (6.5, -4.0, -2.5),
                (Mammoth, _) => (10.0, -5.0, -5.0),
                (Ngoubou, _) => (7.5, -4.0, -1.5),
                (Llama, _) => (5.0, -1.5, -1.0),
                (Alpaca, _) => (3.5, -2.5, -0.5),
                (Akhlut, _) => (8.0, -2.0, 0.5),
                (Bristleback, _) => (6.0, 1.0, -2.0),
                (ClaySteed, _) => (4.0, -1.5, -2.0),
            },
            leg_b: match (body.species, body.body_type) {
                (Grolgar, _) => (6.0, -6.5, -4.0),
                (Saber, _) => (6.0, -7.0, -3.5),
                (Tuskram, _) => (6.0, -5.5, -2.5),
                (Lion, Male) => (6.0, -5.0, -1.5),
                (Lion, Female) => (6.0, -5.0, -1.5),
                (Tarasque, _) => (6.0, -6.5, -6.5),
                (Tiger, _) => (6.0, -7.0, -1.0),
                (Wolf, _) => (5.0, -6.5, -3.0),
                (Frostfang, _) => (3.5, -4.5, -2.0),
                (Mouflon, _) => (3.5, -8.0, -3.5),
                (Catoblepas, _) => (6.0, -2.5, -2.5),
                (Bonerattler, _) => (6.0, -8.0, -2.5),
                (Deer, _) => (3.0, -6.5, -3.5),
                (Hirdrasil, _) => (4.0, -6.5, -3.0),
                (Roshwalr, _) => (7.0, -7.0, -2.5),
                (Donkey, _) => (4.0, -9.0, -3.0),
                (Camel, _) => (4.5, -10.5, -5.0),
                (Zebra, _) => (3.5, -8.0, -3.5),
                (Antelope, _) => (3.5, -7.5, -3.5),
                (Kelpie, _) => (3.5, -7.0, -2.5),
                (Horse, _) => (3.5, -7.0, -2.0),
                (Barghest, _) => (7.0, -3.5, -5.5),
                (Cattle, Male) => (4.0, -7.0, -1.0),
                (Cattle, Female) => (4.0, -6.5, 0.0),
                (Darkhound, _) => (4.0, -6.5, -3.0),
                (Highland, _) => (4.5, -7.0, 0.0),
                (Yak, _) => (4.5, -6.0, -1.0),
                (Panda, _) => (7.0, -7.0, -2.0),
                (Bear, _) => (6.5, -6.5, -2.0),
                (Dreadhorn, _) => (6.0, 0.0, -3.0),
                (Moose, _) => (4.5, -10.0, -2.0),
                (Snowleopard, _) => (5.5, -5.0, -1.5),
                (Mammoth, _) => (7.5, -7.0, -5.0),
                (Ngoubou, _) => (4.5, -9.5, 0.0),
                (Llama, _) => (5.0, -7.0, -2.0),
                (Alpaca, _) => (3.5, -7.0, 0.0),
                (Akhlut, _) => (6.0, -7.5, -2.0),
                (Bristleback, _) => (4.5, -3.0, -2.0),
                (ClaySteed, _) => (4.5, -8.0, -3.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Grolgar, _) => (0.0, 0.0, -4.0),
                (Saber, _) => (1.0, -3.5, -2.5),
                (Tuskram, _) => (-1.0, -1.5, -6.0),
                (Lion, Male) => (0.5, 0.5, -3.5),
                (Lion, Female) => (0.5, 0.5, -3.5),
                (Tarasque, _) => (1.0, 0.0, -3.0),
                (Tiger, _) => (0.5, 0.0, -4.5),
                (Wolf, _) => (0.5, 0.0, -2.0),
                (Frostfang, _) => (0.5, 1.5, -3.5),
                (Mouflon, _) => (-0.5, -0.5, -3.0),
                (Catoblepas, _) => (1.0, 0.0, -6.0),
                (Bonerattler, _) => (-0.5, -3.0, -2.5),
                (Deer, _) => (-0.5, -0.5, -2.5),
                (Hirdrasil, _) => (-0.5, -3.0, -3.5),
                (Roshwalr, _) => (0.5, 0.0, -3.0),
                (Donkey, _) => (0.5, 1.0, -3.5),
                (Camel, _) => (0.0, 0.0, -8.0),
                (Zebra, _) => (-0.5, 0.5, -4.0),
                (Antelope, _) => (-0.5, 0.0, -3.5),
                (Kelpie, _) => (-0.5, 0.5, -4.5),
                (Horse, _) => (-0.5, 0.5, -5.0),
                (Barghest, _) => (2.0, 2.5, -6.0),
                (Cattle, Male) => (-0.5, 1.0, -5.0),
                (Cattle, Female) => (-0.5, 0.5, -5.5),
                (Darkhound, _) => (0.0, 0.5, -4.0),
                (Highland, _) => (-0.5, 0.5, -4.5),
                (Yak, _) => (-0.5, 0.0, -5.0),
                (Panda, _) => (-1.0, 2.0, -4.5),
                (Bear, _) => (0.0, 2.0, -5.5),
                (Dreadhorn, _) => (-0.5, 0.5, -5.0),
                (Moose, _) => (-1.0, 1.5, -9.5),
                (Snowleopard, _) => (0.5, 0.5, -4.5),
                (Mammoth, _) => (-0.5, -0.5, -6.0),
                (Ngoubou, _) => (-1.0, 0.5, -6.0),
                (Llama, _) => (-0.5, 0.5, -6.0),
                (Alpaca, _) => (0.0, -0.5, -5.0),
                (Akhlut, _) => (0.0, 0.0, -5.0),
                (Bristleback, _) => (0.0, -0.5, -2.0),
                (ClaySteed, _) => (-0.5, 0.0, -6.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Grolgar, _) => (0.5, -1.5, -3.0),
                (Saber, _) => (1.0, -1.0, -1.0),
                (Tuskram, _) => (0.5, -1.0, -3.0),
                (Lion, Male) => (0.5, -1.0, -3.0),
                (Lion, Female) => (0.5, -1.0, -3.0),
                (Tarasque, _) => (1.5, -1.0, -2.5),
                (Tiger, _) => (0.5, -1.0, -4.0),
                (Wolf, _) => (0.0, -1.0, -1.5),
                (Frostfang, _) => (0.0, -1.5, -3.5),
                (Mouflon, _) => (-1.0, 0.0, -0.5),
                (Catoblepas, _) => (0.5, 0.5, -4.0),
                (Bonerattler, _) => (0.0, 3.0, -2.5),
                (Deer, _) => (-1.0, -0.5, -2.0),
                (Hirdrasil, _) => (-1.0, -2.0, -4.5),
                (Roshwalr, _) => (0.5, -1.0, -3.5),
                (Donkey, _) => (0.5, -1.0, -3.5),
                (Camel, _) => (0.0, 0.5, -9.0),
                (Zebra, _) => (0.5, -1.0, -3.0),
                (Antelope, _) => (-0.5, -1.5, -3.5),
                (Kelpie, _) => (0.5, -0.5, -3.5),
                (Horse, _) => (0.5, -1.5, -3.5),
                (Barghest, _) => (0.5, 1.0, -4.5),
                (Cattle, Male) => (-0.5, -0.5, -5.0),
                (Cattle, Female) => (-0.5, -1.0, -3.5),
                (Darkhound, _) => (0.0, -1.0, -3.5),
                (Highland, _) => (-0.5, -0.5, -3.0),
                (Yak, _) => (-0.5, -0.5, -5.0),
                (Panda, _) => (-0.5, 0.5, -5.0),
                (Bear, _) => (0.5, 0.5, -6.0),
                (Dreadhorn, _) => (-0.5, 0.5, -3.5),
                (Moose, _) => (-1.0, 0.0, -6.5),
                (Snowleopard, _) => (0.5, 0.5, -5.5),
                (Mammoth, _) => (0.5, -0.5, -4.5),
                (Ngoubou, _) => (0.5, 1.0, -5.5),
                (Llama, _) => (0.5, -1.5, -3.5),
                (Alpaca, _) => (-0.5, -0.5, -5.5),
                (Akhlut, _) => (1.5, -1.0, -4.5),
                (Bristleback, _) => (-0.5, 0.0, -4.0),
                (ClaySteed, _) => (0.0, -0.5, -4.0),
            },
            scaler: match (body.species, body.body_type) {
                (Grolgar, _) => 1.05,
                (Saber, _) => 0.9,
                (Tuskram, _) => 0.95,
                (Lion, Male) => 1.05,
                (Lion, Female) => 1.05,
                (Tarasque, _) => 1.05,
                (Tiger, _) => 0.95,
                (Catoblepas, _) => 1.05,
                (Roshwalr, _) => 1.75,
                (Barghest, _) => 1.2,
                (Antelope, _) => 0.95,
                (Kelpie, _) => 1.1,
                (Donkey, _) => 0.95,
                (Horse, _) => 1.2,
                (Zebra, _) => 1.05,
                (Cattle, _) => 1.25,
                (Highland, _) => 1.32,
                (Bear, _) => 1.4,
                (Yak, _) => 1.4,
                (Camel, _) => 1.15,
                (Dreadhorn, _) => 1.6,
                (Moose, _) => 0.95,
                (Snowleopard, _) => 0.95,
                (Mammoth, _) => 3.0,
                (Ngoubou, _) => 1.0,
                (Akhlut, _) => 1.4,
                (Bristleback, _) => 1.1,
                (ClaySteed, _) => 1.75,
                (Frostfang, _) => 1.0,
                _ => 0.9,
            },
            startangle: match (body.species, body.body_type) {
                //changes the default angle of front feet
                (Grolgar, _) => -0.3,
                (Saber, _) => -0.2,
                (Tuskram, _) => 0.3,
                (Lion, Male) => -0.1,
                (Lion, Female) => -0.1,
                (Tarasque, _) => -0.5,
                (Catoblepas, _) => -0.5,
                (Bonerattler, _) => -0.7,
                (Roshwalr, _) => -0.3,
                (Barghest, _) => -0.5,
                _ => 0.0,
            },
            tempo: match (body.species, body.body_type) {
                (Grolgar, _) => 0.85,
                (Saber, _) => 1.1,
                (Tuskram, _) => 0.9,
                (Lion, Male) => 0.95,
                (Lion, Female) => 0.95,
                (Tarasque, _) => 0.95,
                (Wolf, _) => 1.1,
                (Mouflon, _) => 0.85,
                (Catoblepas, _) => 1.1,
                (Deer, _) => 0.85,
                (Hirdrasil, _) => 0.85,
                (Roshwalr, _) => 0.75,
                (Donkey, _) => 0.85,
                (Zebra, _) => 0.85,
                (Kelpie, _) => 0.85,
                (Horse, _) => 0.85,
                (Barghest, _) => 0.95,
                (Darkhound, _) => 1.1,
                (Cattle, _) => 0.8,
                (Highland, _) => 0.8,
                (Bear, _) => 0.8,
                (Yak, _) => 0.8,
                (Camel, _) => 1.8,
                (Akhlut, _) => 0.95,
                _ => 1.0,
            },
            spring: match (body.species, body.body_type) {
                (Grolgar, _) => 0.9,
                (Saber, _) => 0.9,
                (Tuskram, _) => 0.9,
                (Wolf, _) => 1.2,
                (Mouflon, _) => 0.9,
                (Catoblepas, _) => 0.55,
                (Bonerattler, _) => 1.1,
                (Deer, _) => 0.9,
                (Hirdrasil, _) => 1.1,
                (Donkey, _) => 0.85,
                (Camel, _) => 0.85,
                (Zebra, _) => 0.85,
                (Antelope, _) => 1.2,
                (Kelpie, _) => 0.95,
                (Horse, _) => 0.85,
                (Darkhound, _) => 1.2,
                (Dreadhorn, _) => 0.85,
                (Moose, _) => 0.9,
                (Snowleopard, _) => 1.1,
                _ => 1.0,
            },
            feed: match (body.species, body.body_type) {
                // TODO: Rework some species to allow for feed anim
                (Tuskram, _) => (true, 0.5),
                (Mouflon, _) => (true, 0.7),
                (Deer, _) => (true, 1.0),
                (Hirdrasil, _) => (true, 0.9),
                (Donkey, _) => (false, 1.0),
                (Zebra, _) => (true, 1.0),
                (Antelope, _) => (false, 0.9),
                (Kelpie, _) => (false, 1.0),
                (Horse, _) => (true, 0.85),
                _ => (false, 0.0),
            },
        }
    }
}

fn mount_point(body: &Body) -> Vec3<f32> {
    use comp::quadruped_medium::{BodyType::*, Species::*};
    match (body.species, body.body_type) {
        (Grolgar, _) => (0.0, -6.0, 5.0),
        (Saber, _) => (0.0, -17.0, 2.5),
        (Tuskram, _) => (0.0, -17.0, -1.0),
        (Lion, Male) => (0.0, -8.0, 1.0),
        (Lion, Female) => (0.0, -8.0, 1.0),
        (Tarasque, _) => (0.0, -6.0, 1.0),
        (Tiger, _) => (0.0, -8.0, 2.0),
        (Wolf, _) => (0.0, -9.0, 1.5),
        (Frostfang, _) => (0.0, -6.0, -1.0),
        (Mouflon, _) => (0.0, -8.0, -1.0),
        (Catoblepas, _) => (0.0, -8.0, -1.0),
        (Bonerattler, _) => (0.0, -1.0, 1.0),
        (Deer, _) => (0.0, -9.0, 0.0),
        (Hirdrasil, _) => (0.0, -11.0, 0.0),
        (Roshwalr, _) => (0.0, -1.0, 4.0),
        (Donkey, _) => (0.0, -5.0, -1.0),
        (Camel, _) => (0.0, -13.0, 2.0),
        (Zebra, _) => (0.0, -6.0, 0.0),
        (Antelope, _) => (0.0, -8.0, 0.0),
        (Kelpie, _) => (0.0, -6.0, 0.0),
        (Horse, _) => (0.0, -8.0, 0.0),
        (Barghest, _) => (0.0, -8.0, 2.0),
        (Cattle, Male) => (0.0, -3.0, 5.0),
        (Cattle, Female) => (0.0, -2.0, 3.0),
        (Darkhound, _) => (0.0, -2.0, 0.0),
        (Highland, _) => (0.0, -3.0, 5.0),
        (Yak, _) => (0.0, -8.0, 6.0),
        (Panda, _) => (0.0, -10.0, 2.0),
        (Bear, _) => (0.0, -11.0, 3.0),
        (Dreadhorn, _) => (0.0, 0.0, 7.0),
        (Moose, _) => (0.0, -9.0, 3.0),
        (Snowleopard, _) => (0.0, -9.0, 1.0),
        (Mammoth, _) => (0.0, 5.0, 5.0),
        (Ngoubou, _) => (0.0, -7.0, 3.0),
        (Llama, _) => (0.0, -6.0, 2.0),
        (Alpaca, _) => (0.0, -9.0, 0.0),
        (Akhlut, _) => (0.0, -6.0, 1.0),
        (Bristleback, _) => (0.0, -9.0, 3.0),
        (ClaySteed, _) => (0.0, -6.0, 2.0),
    }
    .into()
}

pub fn quadruped_medium_alpha(
    next: &mut QuadrupedMediumSkeleton,
    s_a: &SkeletonAttr,
    speed: f32,
    stage_section: StageSection,
    anim_time: f32,
    global_time: f32,
    timer: f32,
) {
    let speed = (Vec2::<f32>::from(speed).magnitude()).min(24.0);

    let (movement1base, movement2base, movement3) = match stage_section {
        StageSection::Buildup => (anim_time.powf(0.25), 0.0, 0.0),
        StageSection::Action => (1.0, anim_time.powf(0.25), 0.0),
        StageSection::Recover => (1.0, 1.0, anim_time.powi(4)),
        _ => (0.0, 0.0, 0.0),
    };
    let pullback = 1.0 - movement3;
    let subtract = global_time - timer;
    let check = subtract - subtract.trunc();
    let mirror = (check - 0.5).signum();
    let movement1 = movement1base * mirror * pullback;
    let movement1abs = movement1base * pullback;
    let movement2 = movement2base * mirror * pullback;
    let movement2abs = movement2base * pullback;
    let twitch1 = (movement1 * 10.0).sin() * pullback;
    let twitch2 = (movement3 * 5.0).sin() * pullback;
    let twitchmovement = twitch1 + twitch2;

    next.head.orientation = Quaternion::rotation_x(movement1abs * -0.3 + movement2abs * 0.6)
        * Quaternion::rotation_y(movement1 * 0.35 + movement2 * -0.15)
        * Quaternion::rotation_z(movement1 * 0.15 + movement2 * -0.5);

    next.neck.orientation = Quaternion::rotation_x(movement1abs * 0.2 + movement2abs * -0.2)
        * Quaternion::rotation_y(movement1 * 0.0)
        * Quaternion::rotation_z(movement1 * 0.10 + movement1 * -0.15);

    next.jaw.orientation = Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * 0.4);

    next.tail.orientation =
        Quaternion::rotation_z(movement1 * 0.5 + movement2 * -0.8 + twitchmovement * 0.2 * mirror);
    next.torso_front.position = Vec3::new(
        0.0,
        s_a.torso_front.0 + movement1abs * -4.0,
        s_a.torso_front.1,
    );
    next.torso_front.orientation = Quaternion::rotation_y(movement1 * -0.25 * movement2 * 0.25)
        * Quaternion::rotation_z(movement1 * 0.35 + movement2 * -0.45);

    next.torso_back.orientation = Quaternion::rotation_y(movement1 * 0.25 + movement1 * -0.25)
        * Quaternion::rotation_z(movement1 * -0.4 + movement2 * 0.65);

    next.ears.orientation = Quaternion::rotation_x(twitchmovement * 0.2);
    if speed < 0.5 {
        next.leg_fl.orientation = Quaternion::rotation_x(movement1abs * 0.8 + movement2abs * -0.6)
            * Quaternion::rotation_y(movement1 * -0.3 + movement2 * 0.3)
            * Quaternion::rotation_z(movement1 * -0.35 + movement2 * 0.45);

        next.leg_fr.orientation = Quaternion::rotation_x(movement1abs * 0.8 + movement2abs * -0.6)
            * Quaternion::rotation_y(movement1 * -0.3 + movement2 * 0.3)
            * Quaternion::rotation_z(movement1 * -0.35 + movement2 * 0.45);

        next.leg_bl.orientation = Quaternion::rotation_x(movement1 * 0.1 + movement2 * -0.3);

        next.leg_br.orientation = Quaternion::rotation_x(movement1 * -0.1 + movement2 * 0.3);

        next.foot_fl.orientation = Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.6);

        next.foot_fr.orientation = Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.6);

        next.foot_bl.orientation =
            Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.3);

        next.foot_br.orientation =
            Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.3);
    };
}

pub fn quadruped_medium_beta(
    next: &mut QuadrupedMediumSkeleton,
    s_a: &SkeletonAttr,
    speed: f32,
    stage_section: StageSection,
    anim_time: f32,
    global_time: f32,
    timer: f32,
) {
    let speed = (Vec2::<f32>::from(speed).magnitude()).min(24.0);

    let (movement1base, movement2base, movement3) = match stage_section {
        StageSection::Buildup => (anim_time.powf(0.25), 0.0, 0.0),
        StageSection::Action => (1.0, anim_time.sqrt(), 0.0),
        StageSection::Recover => (1.0, 1.0, anim_time.powi(4)),
        _ => (0.0, 0.0, 0.0),
    };
    let pullback = 1.0 - movement3;
    let subtract = global_time - timer;
    let check = subtract - subtract.trunc();
    let mirror = (check - 0.5).signum();
    let movement1 = movement1base * mirror * pullback;
    let movement1abs = movement1base * pullback;
    let movement2 = movement2base * mirror * pullback;
    let movement2abs = movement2base * pullback;
    let twitch1 = (movement1 * 10.0).sin() * pullback;
    let twitch2 = (movement2abs * -8.0).sin();
    let twitchmovement = twitch1 + twitch2;

    next.head.orientation = Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * 1.1)
        * Quaternion::rotation_y(movement1 * -0.35 + movement2 * 0.25)
        * Quaternion::rotation_z(movement1 * -0.25 + movement2 * 0.5);

    next.neck.orientation = Quaternion::rotation_x(movement1abs * 0.0 + movement2abs * -0.2)
        * Quaternion::rotation_y(movement1 * 0.0)
        * Quaternion::rotation_z(movement1 * -0.10 + movement1 * 0.15);

    next.jaw.orientation = Quaternion::rotation_x(movement1abs * -0.5 + twitch2 * -0.4);

    next.tail.orientation =
        Quaternion::rotation_z(movement1 * 0.5 + movement2 * -0.8 + twitchmovement * 0.2 * mirror);
    next.torso_front.position = Vec3::new(
        0.0,
        s_a.torso_front.0 + movement1abs * -4.0,
        s_a.torso_front.1,
    );
    next.torso_front.orientation = Quaternion::rotation_y(movement1 * -0.25 * movement2 * 0.25)
        * Quaternion::rotation_z(movement1 * 0.35 + movement2 * -0.45);

    next.torso_back.orientation = Quaternion::rotation_y(movement1 * 0.25 + movement1 * -0.25)
        * Quaternion::rotation_z(movement1 * -0.4 + movement2 * 0.65);

    next.ears.orientation = Quaternion::rotation_x(twitchmovement * 0.2);
    if speed < 0.5 {
        next.leg_fl.orientation = Quaternion::rotation_x(movement1abs * 0.8 + movement2abs * -0.6)
            * Quaternion::rotation_y(movement1 * -0.3 + movement2 * 0.3)
            * Quaternion::rotation_z(movement1 * -0.35 + movement2 * 0.45);

        next.leg_fr.orientation = Quaternion::rotation_x(movement1abs * 0.8 + movement2abs * -0.6)
            * Quaternion::rotation_y(movement1 * -0.3 + movement2 * 0.3)
            * Quaternion::rotation_z(movement1 * -0.35 + movement2 * 0.45);

        next.leg_bl.orientation = Quaternion::rotation_x(movement1 * 0.1 + movement2 * -0.3);

        next.leg_br.orientation = Quaternion::rotation_x(movement1 * -0.1 + movement2 * 0.3);

        next.foot_fl.orientation = Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.6);

        next.foot_fr.orientation = Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.6);

        next.foot_bl.orientation =
            Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.3);

        next.foot_br.orientation =
            Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.3);
    };
}
