use crate::{chunk_serialize::ChunkSendQueue, client::Client, presence::Presence};
use common::{comp::Pos, terrain::TerrainGrid};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{CompressedData, ServerGeneral};
use common_state::TerrainChanges;
use specs::{Join, Read, ReadExpect, ReadStorage, WriteStorage};

/// This systems sends new chunks to clients as well as changes to existing
/// chunks
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadExpect<'a, TerrainGrid>,
        Read<'a, TerrainChanges>,
        WriteStorage<'a, ChunkSendQueue>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
    );

    const NAME: &'static str = "terrain_sync";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (terrain, terrain_changes, mut chunk_send_queues, positions, presences, clients): Self::SystemData,
    ) {
        // Sync changed chunks
        for chunk_key in &terrain_changes.modified_chunks {
            for (presence, pos, chunk_send_queue) in
                (&presences, &positions, &mut chunk_send_queues).join()
            {
                if super::terrain::chunk_in_vd(pos.0, *chunk_key, &terrain, presence.view_distance)
                {
                    chunk_send_queue.chunks.push(*chunk_key);
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
