use crate::{
    chunk_serialize::{ChunkSendEntry, SerializedChunk},
    client::Client,
    metrics::NetworkRequestMetrics,
    presence::Presence,
    Tick,
};
use common::{event::EventBus, slowjob::SlowJobPool, terrain::TerrainGrid};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{SerializedTerrainChunk, ServerGeneral};
use hashbrown::{hash_map::Entry, HashMap};
use network::StreamParams;
use specs::{Entity, Read, ReadExpect, ReadStorage};
use std::sync::Arc;

/// This system will handle sending terrain to clients by
/// collecting chunks that need to be send for a single generation run and then
/// trigger a SlowJob for serialisation.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, Tick>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, Presence>,
        ReadExpect<'a, EventBus<ChunkSendEntry>>,
        ReadExpect<'a, NetworkRequestMetrics>,
        ReadExpect<'a, SlowJobPool>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, crossbeam_channel::Sender<SerializedChunk>>,
    );

    const NAME: &'static str = "chunk_serialize";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            tick,
            clients,
            presences,
            chunk_send_queues_bus,
            network_metrics,
            slow_jobs,
            terrain,
            chunk_sender,
        ): Self::SystemData,
    ) {
        // Only operate twice per second
        //TODO: move out of this system and now even spawn this.
        if tick.0.rem_euclid(15) != 0 {
            return;
        }

        struct Metadata {
            recipients: Vec<Entity>,
            lossy_compression: bool,
            params: StreamParams,
        }

        // collect all deduped entities that request a chunk
        let mut chunks = HashMap::<_, Metadata>::new();
        let mut requests = 0u64;
        let mut distinct_requests = 0u64;

        for queue_entry in chunk_send_queues_bus.recv_all() {
            let entry = chunks.entry(queue_entry.chunk_key);
            let meta = match entry {
                Entry::Vacant(ve) => {
                    match clients.get(queue_entry.entity).map(|c| c.terrain_params()) {
                        Some(params) => {
                            distinct_requests += 1;
                            ve.insert(Metadata {
                                recipients: Vec::new(),
                                lossy_compression: true,
                                params,
                            })
                        },
                        None => continue,
                    }
                },
                Entry::Occupied(oe) => oe.into_mut(),
            };

            // We decide here, to ONLY send lossy compressed data If all clients want those.
            // If at least 1 client here does not want lossy we don't compress it twice.
            // It would just be too expensive for the server
            meta.lossy_compression = meta.lossy_compression
                && presences
                    .get(queue_entry.entity)
                    .map(|p| p.lossy_terrain_compression)
                    .unwrap_or(true);
            meta.recipients.push(queue_entry.entity);
            requests += 1;
        }

        network_metrics
            .chunks_serialisation_requests
            .inc_by(requests);
        network_metrics
            .chunks_distinct_serialisation_requests
            .inc_by(distinct_requests);

        // Trigger serialization in a SlowJob
        const CHUNK_SIZE: usize = 10; // trigger one job per 10 chunks to reduce SlowJob overhead. as we use a channel, there is no disadvantage to this
        let mut chunks_iter = chunks
            .into_iter()
            .filter_map(|(chunk_key, meta)| {
                terrain
                    .get_key_arc_real(chunk_key)
                    .map(|chunk| (Arc::clone(chunk), chunk_key, meta))
            })
            .into_iter()
            .peekable();

        while chunks_iter.peek().is_some() {
            let chunks: Vec<_> = chunks_iter.by_ref().take(CHUNK_SIZE).collect();
            let chunk_sender = chunk_sender.clone();
            slow_jobs.spawn("CHUNK_SERIALIZER", move || {
                for (chunk, chunk_key, mut meta) in chunks {
                    let msg = Client::prepare_chunk_update_msg(
                        ServerGeneral::TerrainChunkUpdate {
                            key: chunk_key,
                            chunk: Ok(SerializedTerrainChunk::via_heuristic(
                                &chunk,
                                meta.lossy_compression,
                            )),
                        },
                        &meta.params,
                    );
                    meta.recipients.sort_unstable();
                    meta.recipients.dedup();
                    if let Err(e) = chunk_sender.send(SerializedChunk {
                        lossy_compression: meta.lossy_compression,
                        msg,
                        recipients: meta.recipients,
                    }) {
                        tracing::warn!(?e, "cannot send serialized chunk to sender");
                        break;
                    };
                }
            });
        }
    }
}
