use crate::{event::OnTick, RtState, Rule, RuleError};
use rand::prelude::*;
use rand_chacha::ChaChaRng;

/// Prevent performing cleanup for every NPC every tick
const NPC_SENTIMENT_TICK_SKIP: u64 = 30;
const NPC_CLEANUP_TICK_SKIP: u64 = 100;
const FACTION_CLEANUP_TICK_SKIP: u64 = 30;
const SITE_CLEANUP_TICK_SKIP: u64 = 30;

/// A rule that cleans up data structures in rtsim: removing old reports,
/// irrelevant sentiments, etc.
///
/// Also performs sentiment decay (although this should be moved elsewhere)
pub struct CleanUp;

impl Rule for CleanUp {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnTick>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            let mut rng = ChaChaRng::from_seed(thread_rng().gen::<[u8; 32]>());

            for (_, npc) in data.npcs
                .iter_mut()
                // Only cleanup NPCs every few ticks
                .filter(|(_, npc)| (npc.seed as u64 + ctx.event.tick) % NPC_SENTIMENT_TICK_SKIP == 0)
            {
                npc.sentiments.decay(&mut rng, ctx.event.dt * NPC_SENTIMENT_TICK_SKIP as f32);
            }

            // Clean up entities
            data.npcs
                .iter_mut()
                .filter(|(_, npc)| (npc.seed as u64 + ctx.event.tick) % NPC_CLEANUP_TICK_SKIP == 0)
                .for_each(|(_, npc)| npc.cleanup(&data.reports));

            // Clean up factions
            data.factions
                .iter_mut()
                .filter(|(_, faction)| (faction.seed as u64 + ctx.event.tick) % FACTION_CLEANUP_TICK_SKIP == 0)
                .for_each(|(_, faction)| faction.cleanup());

            // Clean up sites
            data.sites
                .iter_mut()
                .filter(|(_, site)| (site.seed as u64 + ctx.event.tick) % SITE_CLEANUP_TICK_SKIP == 0)
                .for_each(|(_, site)| site.cleanup(&data.reports));

            // Clean up old reports
            data.reports.cleanup(data.time_of_day);
        });

        Ok(Self)
    }
}
