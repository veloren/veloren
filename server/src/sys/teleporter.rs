use common::{
    comp::{Agent, ForceUpdate, Player, Pos, Teleporter},
    CachedSpatialGrid,
};
use common_ecs::{Origin, Phase, System};
use specs::{Entities, Join, Read, ReadStorage, WriteStorage};

const TELEPORT_RADIUS: f32 = 1.;
const MAX_AGGRO_DIST: f32 = 200.; // If an entity further than this is aggroed at a player, the portal will still work

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Pos>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Teleporter>,
        ReadStorage<'a, Agent>,
        WriteStorage<'a, ForceUpdate>,
        Read<'a, CachedSpatialGrid>,
    );

    const NAME: &'static str = "teleporter";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (
            entities,
            mut positions,
            players,
            teleporters,
            agent,
            mut forced_update,
            spatial_grid,
        ): Self::SystemData,
    ) {
        let mut attempt_teleport = vec![];
        let mut player_data = (&entities, &positions, &players).join();

        for (_entity, teleporter_pos, teleporter) in (&entities, &positions, &teleporters).join() {
            let nearby_entities = spatial_grid
                .0
                .in_circle_aabr(teleporter_pos.0.xy(), TELEPORT_RADIUS);

            for (entity, position, _) in nearby_entities.filter_map(|entity| {
                player_data
                    .get(entity, &entities)
                    .filter(|(_, player_pos, _)| {
                        player_pos.0.distance_squared(teleporter_pos.0) <= (TELEPORT_RADIUS).powi(2)
                    })
            }) {
                // TODO: Check for aggro
                attempt_teleport.push((entity, position.0, teleporter))
            }
        }

        for (entity, origin_pos, teleporter) in attempt_teleport {
            if teleporter.requires_no_aggro {
                // FIXME: How does this go with performance?
                let is_aggroed = spatial_grid
                    .0
                    .in_circle_aabr(origin_pos.xy(), MAX_AGGRO_DIST)
                    .any(|agent_entity| {
                        agent.get(agent_entity).map_or(false, |agent| {
                            agent.target.map_or(false, |agent_target| {
                                agent_target.target == entity && agent_target.aggro_on
                            })
                        })
                    });

                if is_aggroed {
                    continue;
                }
            }
            positions
                .get_mut(entity)
                .map(|position| position.0 = teleporter.target);
            forced_update
                .get_mut(entity)
                .map(|forced_update| forced_update.update());
        }
    }
}
