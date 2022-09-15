use crate::{chunk_serialize::ChunkSendEntry, client::Client, presence::Presence, Settings};
use common::{comp::Pos, event::EventBus};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{CompressedData, ServerGeneral};
use common_state::TerrainChanges;
use rayon::prelude::*;
use specs::{Entities, Join, Read, ReadExpect, ReadStorage};
use std::sync::Arc;
use world::World;

/// This systems sends new chunks to clients as well as changes to existing
/// chunks
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, Arc<World>>,
        Read<'a, Settings>,
        Read<'a, TerrainChanges>,
        ReadExpect<'a, EventBus<ChunkSendEntry>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
    );

    const NAME: &'static str = "terrain_sync";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            world,
            server_settings,
            terrain_changes,
            chunk_send_bus,
            positions,
            presences,
            clients,
        ): Self::SystemData,
    ) {
        let max_view_distance = server_settings.max_view_distance.unwrap_or(u32::MAX);
        let (presences_position_entities, _) = super::terrain::prepare_player_presences(
            &world,
            max_view_distance,
            &entities,
            &positions,
            &presences,
            &clients,
        );
        let real_max_view_distance =
            super::terrain::convert_to_loaded_vd(u32::MAX, max_view_distance);

        // Sync changed chunks
        terrain_changes.modified_chunks.par_iter().for_each_init(
            || chunk_send_bus.emitter(),
            |chunk_send_emitter, &chunk_key| {
                // We only have to check players inside the maximum view distance of the server
                // of our own position.
                //
                // We start by partitioning by X, finding only entities in chunks within the X
                // range of us.  These are guaranteed in bounds due to restrictions on max view
                // distance (namely: the square of any chunk coordinate plus the max view
                // distance along both axes must fit in an i32).
                let min_chunk_x = chunk_key.x - real_max_view_distance;
                let max_chunk_x = chunk_key.x + real_max_view_distance;
                let start = presences_position_entities
                    .partition_point(|((pos, _), _)| i32::from(pos.x) < min_chunk_x);
                // NOTE: We *could* just scan forward until we hit the end, but this way we save
                // a comparison in the inner loop, since also needs to check the
                // list length.  We could also save some time by starting from
                // start rather than end, but the hope is that this way the
                // compiler (and machine) can reorder things so both ends are
                // fetched in parallel; since the vast majority of the time both fetched
                // elements should already be in cache, this should not use any
                // extra memory bandwidth.
                //
                // TODO: Benchmark and figure out whether this is better in practice than just
                // scanning forward.
                let end = presences_position_entities
                    .partition_point(|((pos, _), _)| i32::from(pos.x) < max_chunk_x);
                let interior = &presences_position_entities[start..end];
                interior
                    .iter()
                    .filter(|((player_chunk_pos, player_vd_sqr), _)| {
                        super::terrain::chunk_in_vd(*player_chunk_pos, *player_vd_sqr, chunk_key)
                    })
                    .for_each(|(_, entity)| {
                        chunk_send_emitter.emit(ChunkSendEntry {
                            entity: *entity,
                            chunk_key,
                        });
                    });
            },
        );

        // TODO: Don't send all changed blocks to all clients
        // Sync changed blocks
        if !terrain_changes.modified_blocks.is_empty() {
            let mut lazy_msg = None;
            for (_, client) in (&presences, &clients).join() {
                if lazy_msg.is_none() {
                    lazy_msg = Some(client.prepare(ServerGeneral::TerrainBlockUpdates(
                        CompressedData::compress(&terrain_changes.modified_blocks, 1),
                    )));
                }
                lazy_msg.as_ref().map(|msg| client.send_prepared(msg));
            }
        }
    }
}
