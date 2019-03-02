// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{
    CharacterSkeleton,
    super::Animation,
};

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &mut Self::Skeleton,
        time: f64,
    ) {
        let wave = (time as f32 * 12.0).sin();
	    let wavecos = (time as f32 * 12.0).cos();
        let wave_slow = (time as f32 * 6.0 + PI).sin();
        let wavecos_slow = (time as f32 * 6.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

	    skeleton.head.offset = Vec3::new(0.0, 0.0, 0.0);
        skeleton.head.ori = Quaternion::rotation_x(0.0);

        skeleton.chest.offset = Vec3::new(0.0, 0.0, 0.0);
        skeleton.chest.ori = Quaternion::rotation_x(0.0);

        //skeleton.br_foot.offset = Vec3::new(0.0, wavecos_slow * 1.0, wave_slow * 2.0 + wave_dip * 1.0);
	    //skeleton.br_foot.ori = Quaternion::rotation_x(0.0 + wave_slow * 10.1);

	    skeleton.bl_foot.offset = Vec3::new(0.0, 0.0, 80.0);
        skeleton.bl_foot.ori = Quaternion::rotation_x(wave_slow * 2.0);
        //skeleton.bl_foot.offset = Vec3::new(0.0, wavecos_slow * 1.0, wave_slow * 2.0 + wave_dip * 1.0);
        //skeleton.bl_foot.ori = Quaternion::rotation_x(0.5 + wave_slow * 0.1);

	    //skeleton.r_hand.offset = Vec3::new(0.0, wavecos_slow * 1.0, wave_slow * 2.0 + wave_dip * 1.0);
        //skeleton.r_hand.ori = Quaternion::rotation_x(0.5 + wave_slow * 0.1);

        skeleton.l_hand.offset = Vec3::new(0.0, 0.0, 0.0);
        skeleton.l_hand.ori = Quaternion::rotation_x(wave_slow * 2.0);

        //skeleton.l_hand.offset = Vec3::new(0.0, wavecos_slow * 1.0, wave_slow * 2.0 + wave_dip * 1.0);
	    //skeleton.l_hand.ori = Quaternion::rotation_x(0.5 + wave_slow * 0.1);


    }
}
