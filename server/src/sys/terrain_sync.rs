use super::SysTimer;
use crate::streams::{GetStream, InGameStream};
use common::{
    comp::{Player, Pos},
    msg::ServerGeneral,
    span,
    state::TerrainChanges,
    terrain::TerrainGrid,
};
use specs::{Join, Read, ReadExpect, ReadStorage, System, Write, WriteStorage};

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
        ReadStorage<'a, Player>,
        WriteStorage<'a, InGameStream>,
    );

    fn run(
        &mut self,
        (terrain, terrain_changes, mut timer, positions, players, mut in_game_streams): Self::SystemData,
    ) {
        span!(_guard, "run", "terrain_sync::Sys::run");
        timer.start();

        // Sync changed chunks
        'chunk: for chunk_key in &terrain_changes.modified_chunks {
            let mut lazy_msg = None;

            for (player, pos, in_game_stream) in (&players, &positions, &mut in_game_streams).join()
            {
                if player
                    .view_distance
                    .map(|vd| super::terrain::chunk_in_vd(pos.0, *chunk_key, &terrain, vd))
                    .unwrap_or(false)
                {
                    if lazy_msg.is_none() {
                        lazy_msg =
                            Some(in_game_stream.prepare(&ServerGeneral::TerrainChunkUpdate {
                                key: *chunk_key,
                                chunk: Ok(Box::new(match terrain.get_key(*chunk_key) {
                                    Some(chunk) => chunk.clone(),
                                    None => break 'chunk,
                                })),
                            }));
                    }
                    lazy_msg
                        .as_ref()
                        .map(|ref msg| in_game_stream.0.send_raw(&msg));
                }
            }
        }

        // TODO: Don't send all changed blocks to all clients
        // Sync changed blocks
        let mut lazy_msg = None;
        for (player, in_game_stream) in (&players, &mut in_game_streams).join() {
            if lazy_msg.is_none() {
                lazy_msg = Some(in_game_stream.prepare(&ServerGeneral::TerrainBlockUpdates(
                    terrain_changes.modified_blocks.clone(),
                )));
            }
            if player.view_distance.is_some() {
                lazy_msg
                    .as_ref()
                    .map(|ref msg| in_game_stream.0.send_raw(&msg));
            }
        }

        timer.end();
    }
}
