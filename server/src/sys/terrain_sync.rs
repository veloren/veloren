use super::SysTimer;
use crate::client::Client;
use common::{
    comp::{Player, Pos},
    msg::ServerMsg,
    state::TerrainChanges,
    terrain::TerrainGrid,
};
use specs::{Join, Read, ReadExpect, ReadStorage, System, Write, WriteStorage};

/// This systems sends new chunks to clients as well as changes to existing
/// chunks
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadExpect<'a, TerrainGrid>,
        Read<'a, TerrainChanges>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Player>,
        WriteStorage<'a, Client>,
    );

    fn run(
        &mut self,
        (terrain, terrain_changes, mut timer, positions, players, mut clients): Self::SystemData,
    ) {
        timer.start();

        // Sync changed chunks
        'chunk: for chunk_key in &terrain_changes.modified_chunks {
            for (player, pos, client) in (&players, &positions, &mut clients).join() {
                if player
                    .view_distance
                    .map(|vd| super::terrain::chunk_in_vd(pos.0, *chunk_key, &terrain, vd))
                    .unwrap_or(false)
                {
                    client.notify(ServerMsg::TerrainChunkUpdate {
                        key: *chunk_key,
                        chunk: Ok(Box::new(match terrain.get_key(*chunk_key) {
                            Some(chunk) => chunk.clone(),
                            None => break 'chunk,
                        })),
                    });
                }
            }
        }

        // TODO: Don't send all changed blocks to all clients
        // Sync changed blocks
        let msg = ServerMsg::TerrainBlockUpdates(terrain_changes.modified_blocks.clone());
        for (player, client) in (&players, &mut clients).join() {
            if player.view_distance.is_some() {
                client.notify(msg.clone());
            }
        }

        timer.end();
    }
}
