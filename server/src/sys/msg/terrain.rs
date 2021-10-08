use crate::{client::Client, metrics::NetworkRequestMetrics, presence::Presence, ChunkRequest};
use common::{
    comp::Pos,
    spiral::Spiral2d,
    terrain::{TerrainChunkSize, TerrainGrid},
    vol::RectVolSize,
};
use common_ecs::{Job, Origin, ParMode, Phase, System};
use common_net::msg::{ClientGeneral, SerializedTerrainChunk, ServerGeneral};
use rayon::iter::ParallelIterator;
use specs::{Entities, Join, ParJoin, ReadExpect, ReadStorage, Write};
use tracing::{debug, trace};

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, NetworkRequestMetrics>,
        Write<'a, Vec<ChunkRequest>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
    );

    const NAME: &'static str = "msg::terrain";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut Job<Self>,
        (
            entities,
            terrain,
            network_metrics,
            mut chunk_requests,
            positions,
            presences,
            clients,
        ): Self::SystemData,
    ) {
        job.cpu_stats.measure(ParMode::Rayon);
        let mut new_chunk_requests = (&entities, &clients, (&presences).maybe())
            .par_join()
            .map(|(entity, client, maybe_presence)| {
                let mut chunk_requests = Vec::new();
                let _ = super::try_recv_all(client, 5, |client, msg| {
                    let presence = match maybe_presence {
                        Some(g) => g,
                        None => {
                            debug!(?entity, "client is not in_game, ignoring msg");
                            trace!(?msg, "ignored msg content");
                            if matches!(msg, ClientGeneral::TerrainChunkRequest { .. }) {
                                network_metrics.chunks_request_dropped.inc();
                            }
                            return Ok(());
                        },
                    };
                    match msg {
                        ClientGeneral::TerrainChunkRequest { key } => {
                            let in_vd = if let Some(pos) = positions.get(entity) {
                                pos.0.xy().map(|e| e as f64).distance_squared(
                                    key.map(|e| e as f64 + 0.5)
                                        * TerrainChunkSize::RECT_SIZE.map(|e| e as f64),
                                ) < ((presence.view_distance as f64 - 1.0 + 2.5 * 2.0_f64.sqrt())
                                    * TerrainChunkSize::RECT_SIZE.x as f64)
                                    .powi(2)
                            } else {
                                true
                            };
                            if in_vd {
                                match terrain.get_key_arc(key) {
                                    Some(chunk) => {
                                        network_metrics.chunks_served_from_memory.inc();
                                        client.send(ServerGeneral::TerrainChunkUpdate {
                                            key,
                                            chunk: Ok(SerializedTerrainChunk::via_heuristic(
                                                chunk,
                                                presence.lossy_terrain_compression,
                                            )),
                                        })?;
                                        if presence.lossy_terrain_compression {
                                            network_metrics.chunks_served_lossy.inc();
                                        } else {
                                            network_metrics.chunks_served_lossless.inc();
                                        }
                                    },
                                    None => {
                                        network_metrics.chunks_generation_triggered.inc();
                                        chunk_requests.push(ChunkRequest { entity, key });
                                    },
                                }
                            } else {
                                network_metrics.chunks_request_dropped.inc();
                            }
                        },
                        _ => tracing::error!("not a client_terrain msg"),
                    }
                    Ok(())
                });

                // Load a minimum radius of chunks around each player.
                // This is used to prevent view distance reloading exploits and make sure that
                // entity simulation occurs within a minimum radius around the
                // player.
                if let Some(pos) = positions.get(entity) {
                    let player_chunk = pos
                        .0
                        .xy()
                        .map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
                    for rpos in Spiral2d::new().take((crate::MIN_VD as usize + 1).pow(2)) {
                        let key = player_chunk + rpos;
                        if terrain.get_key(key).is_none() {
                            // TODO: @zesterer do we want to be sending these chunk to the client
                            // even if they aren't requested? If we don't we could replace the
                            // entity here with Option<Entity> and pass in None.
                            chunk_requests.push(ChunkRequest { entity, key });
                        }
                    }
                }

                chunk_requests
            })
            .flatten()
            .collect::<Vec<_>>();

        job.cpu_stats.measure(ParMode::Single);

        chunk_requests.append(&mut new_chunk_requests);
    }
}
