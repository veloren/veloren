use crate::{chunk_serialize::ChunkSendEntry, client::Client, presence::Presence};
use common::{comp::Pos, event::EventBus, terrain::TerrainGrid};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{CompressedData, ServerGeneral};
use common_state::TerrainChanges;
use specs::{Entities, Join, Read, ReadExpect, ReadStorage};

/// This systems sends new chunks to clients as well as changes to existing
/// chunks
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, TerrainGrid>,
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
        (entities, terrain, terrain_changes, chunk_send_bus, positions, presences, clients): Self::SystemData,
    ) {
        let mut chunk_send_emitter = chunk_send_bus.emitter();

        // Sync changed chunks
        for chunk_key in &terrain_changes.modified_chunks {
            for (entity, presence, pos) in (&entities, &presences, &positions).join() {
                if super::terrain::chunk_in_vd(pos.0, *chunk_key, &terrain, presence.view_distance)
                {
                    chunk_send_emitter.emit(ChunkSendEntry {
                        entity,
                        chunk_key: *chunk_key,
                    });
                }
            }
        }

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
