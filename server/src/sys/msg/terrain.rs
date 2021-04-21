use crate::{client::Client, metrics::NetworkRequestMetrics, presence::Presence};
use common::{
    comp::Pos,
    event::{EventBus, ServerEvent},
    terrain::{TerrainChunkSize, TerrainGrid},
    vol::RectVolSize,
};
use common_ecs::{Job, Origin, ParMode, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral};
use rayon::iter::ParallelIterator;
use specs::{Entities, Join, ParJoin, Read, ReadExpect, ReadStorage};
use std::sync::Arc;
use tracing::{debug, trace};

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, NetworkRequestMetrics>,
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
            server_event_bus,
            terrain,
            network_metrics,
            positions,
            presences,
            clients,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_event_bus.emitter();

        job.cpu_stats.measure(ParMode::Rayon);
        let mut events = (&entities, &clients, (&presences).maybe())
            .par_join()
            .map(|(entity, client, maybe_presence)| {
                let mut events = Vec::new();
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
                                            chunk: Ok(Arc::clone(chunk)),
                                        })?
                                    },
                                    None => {
                                        network_metrics.chunks_generation_triggered.inc();
                                        events.push(ServerEvent::ChunkRequest(entity, key));
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
                events
            })
            .flatten()
            .collect::<Vec<_>>();

        job.cpu_stats.measure(ParMode::Single);
        for event in events.drain(..) {
            server_emitter.emit(event);
        }
    }
}
