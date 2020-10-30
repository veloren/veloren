use super::SysTimer;
use crate::{
    presence::Presence,
    streams::{GetStream, InGameStream},
};
use common::{comp::Pos, msg::ServerGeneral, span, state::TerrainChanges, terrain::TerrainGrid};
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
        ReadStorage<'a, Presence>,
        WriteStorage<'a, InGameStream>,
    );

    fn run(
        &mut self,
        (terrain, terrain_changes, mut timer, positions, presences, mut in_game_streams): Self::SystemData,
    ) {
        span!(_guard, "run", "terrain_sync::Sys::run");
        timer.start();

        // Sync changed chunks
        'chunk: for chunk_key in &terrain_changes.modified_chunks {
            let mut lazy_msg = None;

            for (presence, pos, in_game_stream) in
                (&presences, &positions, &mut in_game_streams).join()
            {
                if super::terrain::chunk_in_vd(pos.0, *chunk_key, &terrain, presence.view_distance)
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
        for (_, in_game_stream) in (&presences, &mut in_game_streams).join() {
            if lazy_msg.is_none() {
                lazy_msg = Some(in_game_stream.prepare(&ServerGeneral::TerrainBlockUpdates(
                    terrain_changes.modified_blocks.clone(),
                )));
            }
            lazy_msg
                .as_ref()
                .map(|ref msg| in_game_stream.0.send_raw(&msg));
        }

        timer.end();
    }
}
