use crate::{
    chunk_serialize::ChunkSendQueue, client::Client, metrics::NetworkRequestMetrics,
    presence::Presence, Tick,
};
use common::{slowjob::SlowJobPool, terrain::TerrainGrid};

use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{SerializedTerrainChunk, ServerGeneral};
use hashbrown::HashMap;
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, WriteStorage};
use std::cmp::Ordering;

pub(crate) struct LazyTerrainMessage {
    lazy_msg_lo: Option<crate::client::PreparedMsg>,
    lazy_msg_hi: Option<crate::client::PreparedMsg>,
}

pub const SAFE_ZONE_RADIUS: f32 = 200.0;

impl LazyTerrainMessage {
    pub(crate) fn new() -> Self {
        Self {
            lazy_msg_lo: None,
            lazy_msg_hi: None,
        }
    }

    pub(crate) fn prepare_and_send<
        'a,
        A,
        F: FnOnce() -> Result<&'a common::terrain::TerrainChunk, A>,
    >(
        &mut self,
        network_metrics: &NetworkRequestMetrics,
        client: &Client,
        presence: &Presence,
        chunk_key: &vek::Vec2<i32>,
        generate_chunk: F,
    ) -> Result<(), A> {
        let lazy_msg = if presence.lossy_terrain_compression {
            &mut self.lazy_msg_lo
        } else {
            &mut self.lazy_msg_hi
        };
        if lazy_msg.is_none() {
            *lazy_msg = Some(client.prepare(ServerGeneral::TerrainChunkUpdate {
                key: *chunk_key,
                chunk: Ok(match generate_chunk() {
                    Ok(chunk) => SerializedTerrainChunk::via_heuristic(
                        chunk,
                        presence.lossy_terrain_compression,
                    ),
                    Err(e) => return Err(e),
                }),
            }));
        }
        lazy_msg.as_ref().map(|msg| {
            let _ = client.send_prepared(msg);
            if presence.lossy_terrain_compression {
                network_metrics.chunks_served_lossy.inc();
            } else {
                network_metrics.chunks_served_lossless.inc();
            }
        });
        Ok(())
    }
}

/// This system will handle sending terrain to clients by
/// collecting chunks that need to be send for a single generation run and then
/// trigger a SlowJob for serialisation.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, Tick>,
        Entities<'a>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, Presence>,
        WriteStorage<'a, ChunkSendQueue>,
        ReadExpect<'a, SlowJobPool>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, NetworkRequestMetrics>,
    );

    const NAME: &'static str = "chunk_serialize";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            tick,
            entities,
            clients,
            presences,
            mut chunk_send_queues,
            slow_jobs,
            terrain,
            network_metrics,
        ): Self::SystemData,
    ) {
        // Only operate once per second
        if tick.0.rem_euclid(60) != 0 {
            return;
        }

        let mut chunks = HashMap::<_, Vec<_>>::new();

        for entity in (&entities, &clients, &presences, !&chunk_send_queues)
            .join()
            .map(|(e, _, _, _)| e)
            .collect::<Vec<_>>()
        {
            let _ = chunk_send_queues.insert(entity, ChunkSendQueue::default());
        }

        // Grab all chunk requests for all clients and sort them
        for (entity, _client, chunk_send_queue) in
            (&entities, &clients, &mut chunk_send_queues).join()
        {
            let mut chunk_send_queue = std::mem::take(chunk_send_queue);
            // dedup input
            chunk_send_queue.chunks.sort_by(|a, b| {
                let zero = a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal);
                let one = a.y.partial_cmp(&b.y).unwrap_or(Ordering::Equal);
                if matches!(zero, Ordering::Equal) {
                    one
                } else {
                    zero
                }
            });
            chunk_send_queue.chunks.dedup();
            for chunk_key in chunk_send_queue.chunks {
                let recipients = chunks.entry(chunk_key).or_default();
                recipients.push(entity);
            }
        }

        if !chunks.is_empty() {
            let len = chunks.len();
            print!("{}", len);
            for (chunk_key, entities) in chunks {
                let mut lazy_msg = LazyTerrainMessage::new();
                for entity in entities {
                    let client = clients.get(entity).unwrap();
                    let presence = presences.get(entity).unwrap();
                    if let Err(e) = lazy_msg.prepare_and_send(
                        &network_metrics,
                        client,
                        presence,
                        &chunk_key,
                        || terrain.get_key(chunk_key).ok_or(()),
                    ) {
                        tracing::error!(?e, "error sending chunk");
                    }
                }
            }
        }
    }
}
