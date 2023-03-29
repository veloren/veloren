#[cfg(test)]
mod tests {
    use common::{
        comp::{
            item::MaterialStatManifest, skills::GeneralSkill, tool::AbilityMap, CharacterState,
            Controller, Energy, Ori, PhysicsState, Poise, Pos, Skill, Stats, Vel,
        },
        resources::{DeltaTime, GameMode, Time},
        shared_server_config::ServerConstants,
        terrain::{MapSizeLg, TerrainChunk},
        uid::Uid,
        util::Dir,
        SkillSetBuilder,
    };
    use common_ecs::dispatch;
    use common_state::State;
    use rand::thread_rng;
    use specs::{Builder, Entity, WorldExt};
    use std::{sync::Arc, time::Duration};
    use vek::{approx::AbsDiffEq, Vec2, Vec3};
    use veloren_common_systems::character_behavior;

    const DEFAULT_WORLD_CHUNKS_LG: MapSizeLg =
        if let Ok(map_size_lg) = MapSizeLg::new(Vec2 { x: 1, y: 1 }) {
            map_size_lg
        } else {
            panic!("Default world chunk size does not satisfy required invariants.");
        };

    fn setup() -> State {
        let pools = State::pools(GameMode::Server);
        let mut state = State::new(
            GameMode::Server,
            pools,
            DEFAULT_WORLD_CHUNKS_LG,
            Arc::new(TerrainChunk::water(0)),
        );
        let msm = MaterialStatManifest::load().cloned();
        state.ecs_mut().insert(msm);
        state.ecs_mut().insert(AbilityMap::load().cloned());
        state.ecs_mut().read_resource::<Time>();
        state.ecs_mut().read_resource::<DeltaTime>();
        state
    }

    fn create_entity(state: &mut State, ori: Ori) -> Entity {
        let body = common::comp::Body::Humanoid(common::comp::humanoid::Body::random_with(
            &mut thread_rng(),
            &common::comp::humanoid::Species::Human,
        ));
        let skill_set = SkillSetBuilder::default().build();
        state
            .ecs_mut()
            .create_entity()
            .with(CharacterState::Idle(common::states::idle::Data::default()))
            .with(Pos(Vec3::zero()))
            .with(Vel::default())
            .with(ori)
            .with(body.mass())
            .with(body.density())
            .with(body)
            .with(Energy::new(
                body,
                skill_set
                    .skill_level(Skill::General(GeneralSkill::EnergyIncrease))
                    .unwrap_or(0),
            ))
            .with(Controller::default())
            .with(Poise::new(body))
            .with(skill_set)
            .with(PhysicsState::default())
            .with(Stats::empty(body))
            .with(Uid(1))
            .build()
    }

    fn tick(state: &mut State, dt: Duration) {
        state.tick(
            dt,
            |dispatch_builder| {
                dispatch::<character_behavior::Sys>(dispatch_builder, &[]);
            },
            false,
            None,
            // Dummy ServerConstants
            &ServerConstants::default(),
        );
    }

    #[test]
    fn orientation_shortcut() {
        let mut state = setup();
        const TESTCASES: usize = 5;
        let testcases: [(Vec3<f32>, Vec3<f32>); TESTCASES] = [
            // horizontal is unchanged
            (Vec3::unit_x(), Vec3::unit_x()),
            // nearly vertical takes time to adjust
            (Vec3::new(0.1, 0.1, 1.0), Vec3::new(0.149, 0.149, 0.978)),
            // intermediate case
            (Vec3::new(0.6, 0.6, 0.1), Vec3::new(0.706, 0.706, 0.052)),
            // edge case: nearly horizontal after system
            (Vec3::new(0.6, 0.6, 0.0556), Vec3::new(0.707, 0.707, 0.000)),
            // small enough to be horizontal in one step
            (Vec3::new(0.6, 0.6, 0.04), Vec3::new(0.707, 0.707, 0.000)),
        ];
        let mut entities: [Option<Entity>; TESTCASES] = [None; TESTCASES];
        for i in 0..TESTCASES {
            entities[i] = Some(create_entity(
                &mut state,
                Ori::from_unnormalized_vec(testcases[i].0).unwrap_or_default(),
            ));
        }
        tick(&mut state, Duration::from_secs_f32(0.033));
        let results = state.ecs().read_storage::<Ori>();
        for i in 0..TESTCASES {
            if let Some(e) = entities[i] {
                let result = Dir::from(*results.get(e).expect("Ori missing"));
                assert!(result.abs_diff_eq(&testcases[i].1, 0.0005));
                // println!("{:?}", result);
            }
        }
    }
}
