use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;
pub struct ChargeswingAnimation;

impl Animation for ChargeswingAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_chargeswing\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_chargeswing")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let lab = 1.0;

        // Spin stuff here
        let foot = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 10.32).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 10.32).sin());

        let decel = (anim_time as f32 * 16.0 * lab as f32).min(PI / 2.0).sin();

        let spin = (anim_time as f32 * 2.8 * lab as f32).sin();
        let spinhalf = (anim_time as f32 * 1.4 * lab as f32).sin();
        let short = (((5.0)
            / (1.5
                + 3.5 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        // end spin stuff

        let movement = (anim_time as f32 * 1.0);
        let fire = (anim_time as f32 * 18.0 * lab as f32).sin();

        let foothoril = (anim_time as f32 * 8.0 * lab as f32 + PI * 1.45).sin();
        let foothorir = (anim_time as f32 * 8.0 * lab as f32 + PI * (0.45)).sin();

        let footvertl = (anim_time as f32 * 8.0 * lab as f32).sin();
        let footvertr = (anim_time as f32 * 8.0 * lab as f32 + PI).sin();
        let footrotl = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 8.0 * lab as f32 + PI * 1.4).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 8.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 8.0 * lab as f32 + PI * 0.4).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 8.0* lab as f32 + PI * 0.4).sin());
        if let Some(ToolKind::Hammer(_)) = active_tool_kind {
                next.l_hand.position = Vec3::new(-12.0, 0.0, 0.0);
                next.l_hand.orientation =
                    Quaternion::rotation_x(-0.0) * Quaternion::rotation_y(0.0);
                next.l_hand.scale = Vec3::one() * 1.08;
                next.r_hand.position = Vec3::new(2.0, 0.0, 0.0);
                next.r_hand.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
                next.r_hand.scale = Vec3::one() * 1.06;
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation =
                    Quaternion::rotation_y(-1.57) * Quaternion::rotation_z(1.57);

                next.control.position = Vec3::new(6.0, 7.0, 1.0);
                next.control.orientation = Quaternion::rotation_x(0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.control.scale = Vec3::one();
            if let Some(stage_section) = stage_section {
                match stage_section {
                    StageSection::Charge => {
                next.control.position = Vec3::new(6.0+(movement*-4.0).max(-8.0), 7.0+(movement*2.0).min(2.0), 1.0);
                next.control.orientation = Quaternion::rotation_x(0.3)
                    * Quaternion::rotation_y(0.0+(movement*0.7).min(0.7)+fire*0.1*(anim_time as f32).min(2.0))
                    * Quaternion::rotation_z(0.0+(movement*0.2).min(0.5));    

                next.chest.position = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1);
                next.chest.orientation = Quaternion::rotation_z((movement*2.0).min(PI/2.0));  
                next.shorts.orientation = Quaternion::rotation_z((short*0.25+movement*-1.0).max(-PI/4.0));  

                next.head.position = Vec3::new(0.0, skeleton_attr.head.0-2.0+(movement*2.0).min(2.0), skeleton_attr.head.1);

                next.head.orientation = Quaternion::rotation_z((movement*-1.8).max(PI/-2.0));   
        next.shorts.orientation =
            Quaternion::rotation_z(short * 0.25);
                    //next.l_foot.orientation =
                        //Quaternion::rotation_x((movement*-0.2).max(-0.5))*Quaternion::rotation_z((movement*0.7).min(0.7));
                    //next.r_foot.orientation =
                        //Quaternion::rotation_x((movement*-0.2).max(-0.5))*Quaternion::rotation_z((movement*0.7).min(0.7)); 
if speed > 0.5{

            next.l_foot.position = Vec3::new(
            -skeleton_attr.foot.0,
            -1.5 + skeleton_attr.foot.1 + foothoril * -3.5-2.0,
            2.0 + skeleton_attr.foot.2 + ((footvertl * -1.7).max(-1.0)),
        );        

        next.r_foot.position = Vec3::new(
            skeleton_attr.foot.0,
            -1.5 + skeleton_attr.foot.1 + foothorir * -3.5+5.0,
            2.0 + skeleton_attr.foot.2 + ((footvertr * -1.7).max(-1.0)),
        );

        next.l_foot.orientation = Quaternion::rotation_x(-0.4 + footrotl * -0.2)
            * Quaternion::rotation_z((movement*0.5).min(0.5));
        next.l_foot.scale = Vec3::one();

        next.r_foot.orientation = Quaternion::rotation_x(-0.4 + footrotr * -0.2)
            * Quaternion::rotation_z((movement*0.5).min(0.5));
}
else{

            next.l_foot.position = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1-5.0,
            skeleton_attr.foot.2,
        );        

        next.r_foot.position = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1+7.0,
            skeleton_attr.foot.2,
        );

        next.l_foot.orientation = Quaternion::rotation_x(-0.2);

        next.r_foot.orientation = Quaternion::rotation_x(0.2);


};

                    },

                    StageSection::Swing => {
                next.chest.orientation = Quaternion::rotation_z(-0.5);  
                next.control.position = Vec3::new(6.0, 7.0, 1.0+3.0);
                next.control.orientation = Quaternion::rotation_x(PI/2.0)
                    * Quaternion::rotation_y(-1.6)
                    * Quaternion::rotation_z(-0.1);    
                next.head.orientation = Quaternion::rotation_z(0.4);   

                    },
                    StageSection::Recover => {
                next.chest.orientation = Quaternion::rotation_z(-0.5+movement*0.5);  
                next.control.position = Vec3::new(6.0, 7.0, 1.0+3.0+movement*-3.0);
                next.control.orientation = Quaternion::rotation_x(PI/2.0)
                    * Quaternion::rotation_y(-1.6+movement*1.6)
                    * Quaternion::rotation_z(-0.1+movement*0.1); 
                next.head.orientation = Quaternion::rotation_z(0.4+movement*-0.4);   

                    },
                    _ => {},
                }
            }
        }

        /*        if let Some(ToolKind::Hammer(_)) = active_tool_kind {
                    next.l_hand.position = Vec3::new(-12.0, 0.0, 0.0);
                    next.l_hand.orientation = Quaternion::rotation_x(-0.0) * Quaternion::rotation_y(0.0);
                    next.l_hand.scale = Vec3::one() * 1.08;
                    next.r_hand.position = Vec3::new(3.0, 0.0, 0.0);
                    next.r_hand.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
                    next.r_hand.scale = Vec3::one() * 1.06;
                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0)
                        * Quaternion::rotation_y(-1.57)
                        * Quaternion::rotation_z(1.57);

                    next.head.position = Vec3::new(
                        0.0,
                        -2.0 + skeleton_attr.head.0 + slower * -1.0,
                        skeleton_attr.head.1,
                    );
                    next.head.orientation = Quaternion::rotation_z(slower * 0.05)
                        * Quaternion::rotation_x((slowersmooth * -0.25 + slower * 0.55).max(-0.2))
                        * Quaternion::rotation_y(slower * 0.05);
                    next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                    next.chest.position = Vec3::new(0.0, 0.0, 7.0);
                    next.chest.orientation = Quaternion::rotation_z(slower * 0.08 + slowersmooth * 0.15)
                        * Quaternion::rotation_x(-0.3 + slower * 0.45 + slowersmooth * 0.26)
                        * Quaternion::rotation_y(slower * 0.18 + slowersmooth * 0.15);

                    next.belt.position = Vec3::new(0.0, 0.0, -2.0 + slower * -0.7);
                    next.belt.orientation = Quaternion::rotation_z(slower * -0.16 + slowersmooth * -0.12)
                        * Quaternion::rotation_x(0.0 + slower * -0.06)
                        * Quaternion::rotation_y(slower * -0.05);

                    next.shorts.position = Vec3::new(0.0, 0.0, -5.0 + slower * -0.7);
                    next.shorts.orientation = Quaternion::rotation_z(slower * -0.08 + slowersmooth * -0.08)
                        * Quaternion::rotation_x(0.0 + slower * -0.08 + slowersmooth * -0.08)
                        * Quaternion::rotation_y(slower * -0.07);

                    next.lantern.orientation =
                        Quaternion::rotation_x(slower * -0.7 + 0.4) * Quaternion::rotation_y(slower * 0.4);
                    next.hold.scale = Vec3::one() * 0.0;

                    next.l_foot.position = Vec3::new(
                        -skeleton_attr.foot.0,
                        slower * 3.0 + slowersmooth * -6.0 - 2.0,
                        skeleton_attr.foot.2,
                    );
                    next.l_foot.orientation =
                        Quaternion::rotation_x(slower * -0.2 + slowersmooth * -0.3 - 0.2);

                    next.r_foot.position = Vec3::new(
                        skeleton_attr.foot.0,
                        slower * 2.0 + slowersmooth * -4.0 - 1.0,
                        -2.0 + skeleton_attr.foot.2,
                    );
                    next.r_foot.orientation =
                        Quaternion::rotation_x(slower * -0.4 + slowersmooth * -0.6 - 1.0);

                    next.control.scale = Vec3::one();
                    next.control.position = Vec3::new(-7.0, 7.0, 1.0);
                    next.control.orientation = Quaternion::rotation_x(-0.7 + slower * 1.5)
                        * Quaternion::rotation_y(0.0)
                        * Quaternion::rotation_z(1.4 + slowersmooth * -0.4 + slower * 0.2);
                    next.control.scale = Vec3::one();

                    next.lantern.position = Vec3::new(
                        skeleton_attr.lantern.0,
                        skeleton_attr.lantern.1,
                        skeleton_attr.lantern.2,
                    );
                    next.glider.position = Vec3::new(0.0, 0.0, 10.0);
                    next.glider.scale = Vec3::one() * 0.0;
                    next.l_control.scale = Vec3::one();
                    next.r_control.scale = Vec3::one();

                    next.torso.position = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
                    next.torso.orientation = Quaternion::rotation_z(0.0);
                    next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        */


        //next.lantern.position = Vec3::new(
        //    skeleton_attr.lantern.0,
        //    skeleton_attr.lantern.1,
        //    skeleton_attr.lantern.2,
        //);
        //next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        //next.glider.scale = Vec3::one() * 0.0;
        //next.l_control.scale = Vec3::one();
        //next.r_control.scale = Vec3::one();

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        //next.torso.position = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
        //next.torso.orientation = Quaternion::rotation_z(0.0);
        //next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        next
    }
}
