use crate::{chunk_serialize::SerializedChunk, client::Client, metrics::NetworkRequestMetrics};

use common_ecs::{Job, Origin, Phase, System};
use specs::{ReadExpect, ReadStorage};

/// This system will handle sending terrain to clients by
/// collecting chunks that need to be send for a single generation run and then
/// trigger a SlowJob for serialisation.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Client>,
        ReadExpect<'a, NetworkRequestMetrics>,
        ReadExpect<'a, crossbeam_channel::Receiver<SerializedChunk>>,
    );

    const NAME: &'static str = "chunk_send";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (clients, network_metrics, chunk_receiver): Self::SystemData) {
        let mut lossy = 0u64;
        let mut lossless = 0u64;
        for sc in chunk_receiver.try_iter() {
            for recipient in sc.recipients {
                if let Some(client) = clients.get(recipient) {
                    if client.send_prepared(&sc.msg).is_err() {
                        if sc.lossy_compression {
                            lossy += 1;
                        } else {
                            lossless += 1;
                        }
                    }
                }
            }
        }
        network_metrics.chunks_served_lossy.inc_by(lossy);
        network_metrics.chunks_served_lossless.inc_by(lossless);
    }
}
