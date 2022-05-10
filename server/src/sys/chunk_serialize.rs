use crate::{
    chunk_serialize::{ChunkSendQueue, SerializedChunk},
    client::Client,
    presence::Presence,
    Tick,
};
use common::{slowjob::SlowJobPool, terrain::TerrainGrid};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{SerializedTerrainChunk, ServerGeneral};
use hashbrown::HashMap;
use network::StreamParams;
use specs::{Entities, Entity, Join, Read, ReadExpect, ReadStorage, WriteStorage};
use std::{cmp::Ordering, sync::Arc};

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
        ReadExpect<'a, crossbeam_channel::Sender<SerializedChunk>>,
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
            chunk_sender,
        ): Self::SystemData,
    ) {
        // Only operate once per second
        //TODO: move out of this system and now even spawn this.
        if tick.0.rem_euclid(30) != 0 {
            return;
        }

        for entity in (&entities, &clients, &presences, !&chunk_send_queues)
            .join()
            .map(|(e, _, _, _)| e)
            .collect::<Vec<_>>()
        {
            let _ = chunk_send_queues.insert(entity, ChunkSendQueue::default());
        }

        struct Metadata {
            recipients: Vec<Entity>,
            lossy_compression: bool,
            params: StreamParams,
        }

        let mut chunks = HashMap::<_, Metadata>::new();
        // Grab all chunk requests for all clients and sort them
        for (entity, client, presence, chunk_send_queue) in
            (&entities, &clients, &presences, &mut chunk_send_queues).join()
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
                let meta = chunks.entry(chunk_key).or_insert_with(|| Metadata {
                    recipients: Vec::default(),
                    lossy_compression: true,
                    params: client.terrain_params(),
                });
                meta.recipients.push(entity);
                // We decide here, to ONLY send lossy compressed data If all clients want those.
                // If at least 1 client here does not want lossy we don't compress it twice.
                // It would just be too expensive for the server
                meta.lossy_compression =
                    meta.lossy_compression && presence.lossy_terrain_compression;
            }
        }

        // Trigger serialization in a SlowJob
        for (chunk_key, meta) in chunks {
            if let Some(chunk) = terrain.get_key_arc(chunk_key) {
                let chunk = Arc::clone(chunk);
                let chunk_sender = chunk_sender.clone();
                slow_jobs.spawn("CHUNK_SERIALIZER", move || {
                    let msg = Client::prepare_terrain(
                        ServerGeneral::TerrainChunkUpdate {
                            key: chunk_key,
                            chunk: Ok(SerializedTerrainChunk::via_heuristic(
                                &chunk,
                                meta.lossy_compression,
                            )),
                        },
                        &meta.params,
                    );
                    if let Err(e) = chunk_sender.send(SerializedChunk {
                        lossy_compression: meta.lossy_compression,
                        msg,
                        recipients: meta.recipients,
                    }) {
                        tracing::warn!(?e, "cannot send serialized chunk to sender")
                    };
                });
            }
        }
    }
}
