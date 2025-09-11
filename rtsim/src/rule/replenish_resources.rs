use crate::{RtState, Rule, RuleError, event::OnTick};

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

            let chunks = data.nature.chunks.raw_mut();
            // The number of times we need to replenish to cover the whole world
            let group_count = chunks.len() / REPLENISH_PER_TICK + 1;
            for chunk in chunks
                .iter_mut()
                .skip((ctx.event.tick as usize % group_count) * REPLENISH_PER_TICK)
                .take(REPLENISH_PER_TICK)
            {
                for (_, res) in &mut chunk.res {
                    *res = (*res + replenish_amount).clamp(0.0, 1.0);
                }
            }
        });

        Ok(Self)
    }
}
