use crate::{RtState, Rule, RuleError, event::OnTick};
use rand::prelude::*;

pub struct ReplenishResources;

/// Take 1 hour to replenish resources entirely. Makes farming unviable, but
/// probably still poorly balanced.
// TODO: Different rates for different resources?
// TODO: Non-renewable resources?
pub const REPLENISH_TIME: f32 = 60.0 * 60.0;
/// How many chunks should be replenished per tick?
// TODO: It should be possible to optimise this be remembering the last
// modification time for each chunk, then lazily projecting forward using a
// closed-form solution to the replenishment to calculate resources in a lazy
// manner.
pub const REPLENISH_PER_TICK: usize = 8192;

impl Rule for ReplenishResources {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnTick>(|ctx| {
            let world_size = ctx.world.sim().get_size();
            let mut data = ctx.state.data_mut();

            // How much should be replenished for each chosen chunk to hit our target
            // replenishment rate?
            let replenish_amount = world_size.product() as f32
                * ctx.event.dt
                * (1.0 / REPLENISH_TIME / REPLENISH_PER_TICK as f32);
            for _ in 0..REPLENISH_PER_TICK {
                let key = world_size.map(|e| thread_rng().gen_range(0..e as i32));

                let mut res = data.nature.get_chunk_resources(key);
                for (_, res) in &mut res {
                    *res = (*res + replenish_amount).clamp(0.0, 1.0);
                }
                data.nature.set_chunk_resources(key, res);
            }
        });

        Ok(Self)
    }
}
