use crate::{
    chunk_serialize::ChunkSendEntry, client::Client, lod::Lod, metrics::NetworkRequestMetrics,
    ChunkRequest,
};
use common::{
    comp::{Pos, Presence},
    event::{ClientDisconnectEvent, EventBus},
    spiral::Spiral2d,
    terrain::{CoordinateConversions, TerrainChunkSize, TerrainGrid},
    vol::RectVolSize,
};
use common_ecs::{Job, Origin, ParMode, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral};
use rayon::prelude::*;
use specs::{Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, Write, WriteStorage};
use tracing::{debug, trace};

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ClientDisconnectEvent>>,
        ReadExpect<'a, EventBus<ChunkSendEntry>>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, Lod>,
        ReadExpect<'a, NetworkRequestMetrics>,
        Write<'a, Vec<ChunkRequest>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Presence>,
        WriteStorage<'a, Client>,
    );

    const NAME: &'static str = "msg::terrain";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut Job<Self>,
        (
            entities,
            client_disconnect_events,
            chunk_send_bus,
            terrain,
            lod,
            network_metrics,
            mut chunk_requests,
            positions,
            presences,
            mut clients,
        ): Self::SystemData,
    ) {
        job.cpu_stats.measure(ParMode::Rayon);
        let mut new_chunk_requests = (&entities, &mut clients, (&presences).maybe())
            .join()
            // NOTE: Required because Specs has very poor work splitting for sparse joins.
            .par_bridge()
            .map_init(
                || (chunk_send_bus.emitter(), client_disconnect_events.emitter()),
                |(chunk_send_emitter, client_disconnect_emitter), (entity, client, maybe_presence)| {
                    let mut chunk_requests = Vec::new();
                    let _ = super::try_recv_all(client, 5, |client, msg| {
                        // SPECIAL CASE: LOD zone requests can be sent by non-present players
                        if let ClientGeneral::LodZoneRequest { key } = &msg {
                            client.send(ServerGeneral::LodZoneUpdate {
                                key: *key,
                                zone: lod.zone(*key).clone(),
                            })?;
                        } else {
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
                                        ) < ((presence.terrain_view_distance.current() as f64 - 1.0
                                            + 2.5 * 2.0_f64.sqrt())
                                            * TerrainChunkSize::RECT_SIZE.x as f64)
                                            .powi(2)
                                    } else {
                                        true
                                    };
                                    if in_vd {
                                        if terrain.get_key_arc(key).is_some() {
                                            network_metrics.chunks_served_from_memory.inc();
                                            chunk_send_emitter.emit(ChunkSendEntry {
                                                chunk_key: key,
                                                entity,
                                            });
                                        } else {
                                            network_metrics.chunks_generation_triggered.inc();
                                            chunk_requests.push(ChunkRequest { entity, key });
                                        }
                                    } else {
                                        network_metrics.chunks_request_dropped.inc();
                                    }
                                },
                                _ => {
                                    debug!(
                                        "Kicking possibly misbehaving client due to invalud terrain \
                                         request"
                                    );
                                    client_disconnect_emitter.emit(ClientDisconnectEvent(
                                        entity,
                                        common::comp::DisconnectReason::NetworkError,
                                    ));
                                },
                            }
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
                            .as_::<i32>()
                            .wpos_to_cpos();
                        for rpos in Spiral2d::new().take((crate::MIN_VD as usize + 1).pow(2)) {
                            let key = player_chunk + rpos;
                            if terrain.get_key(key).is_none() {
                                // TODO: @zesterer do we want to be sending these chunk to the
                                // client even if they aren't
                                // requested? If we don't we could replace the
                                // entity here with Option<Entity> and pass in None.
                                chunk_requests.push(ChunkRequest { entity, key });
                            }
                        }
                    }

                    chunk_requests
                },
            )
            .flatten()
            .collect::<Vec<_>>();

        job.cpu_stats.measure(ParMode::Single);

        chunk_requests.append(&mut new_chunk_requests);
    }
}
