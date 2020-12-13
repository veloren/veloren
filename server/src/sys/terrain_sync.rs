use super::SysTimer;
use crate::{client::Client, presence::Presence};
use common::{comp::Pos, span, terrain::TerrainGrid};
use common_net::msg::ServerGeneral;
use common_sys::state::TerrainChanges;
use specs::{Join, Read, ReadExpect, ReadStorage, System, Write};

/// This systems sends new chunks to clients as well as changes to existing
/// chunks
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        ReadExpect<'a, TerrainGrid>,
        Read<'a, TerrainChanges>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
    );

    fn run(
        &mut self,
        (terrain, terrain_changes, mut timer, positions, presences, clients): Self::SystemData,
    ) {
        span!(_guard, "run", "terrain_sync::Sys::run");
        timer.start();

        // Sync changed chunks
        'chunk: for chunk_key in &terrain_changes.modified_chunks {
            let mut lazy_msg = None;

            for (presence, pos, client) in (&presences, &positions, &clients).join() {
                if super::terrain::chunk_in_vd(pos.0, *chunk_key, &terrain, presence.view_distance)
                {
                    if lazy_msg.is_none() {
                        lazy_msg = Some(client.prepare(ServerGeneral::TerrainChunkUpdate {
                            key: *chunk_key,
                            chunk: Ok(Box::new(match terrain.get_key(*chunk_key) {
                                Some(chunk) => chunk.clone(),
                                None => break 'chunk,
                            })),
                        }));
                    }
                    lazy_msg.as_ref().map(|ref msg| client.send_prepared(&msg));
                }
            }
        }

        // TODO: Don't send all changed blocks to all clients
        // Sync changed blocks
        let mut lazy_msg = None;
        for (_, client) in (&presences, &clients).join() {
            if lazy_msg.is_none() {
                lazy_msg = Some(client.prepare(ServerGeneral::TerrainBlockUpdates(
                    terrain_changes.modified_blocks.clone(),
                )));
            }
            lazy_msg.as_ref().map(|ref msg| client.send_prepared(&msg));
        }

        timer.end();
    }
}
