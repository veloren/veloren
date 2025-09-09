use crate::{RtState, Rule, RuleError, event::OnTick};
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
            let mut rng = ChaChaRng::from_seed(rand::rng().random::<[u8; 32]>());

            // TODO: Use `.into_par_iter()` for these by implementing rayon traits in upstream slotmap.

            // Decay NPC sentiments
            data.npcs
                .iter_mut()
                // Only cleanup NPCs every few ticks
                .filter(|(_, npc)| (npc.seed as u64 + ctx.event.tick).is_multiple_of(NPC_SENTIMENT_TICK_SKIP))
                .for_each(|(_, npc)| npc.sentiments.decay(&mut rng, ctx.event.dt * NPC_SENTIMENT_TICK_SKIP as f32));

            // Remove dead NPCs
            // TODO: Don't do this every tick, find a sensible way to gradually remove dead NPCs after they've been
            // forgotten
            data.npcs
                .retain(|npc_id, npc| if npc.is_dead() {
                    // Remove NPC from home population
                    if let Some(home) = npc.home.and_then(|home| data.sites.get_mut(home)) {
                        home.population.remove(&npc_id);
                    }
                    tracing::debug!(?npc_id, "Cleaning up dead NPC");
                    false
                } else {
                    true
                });

            // Clean up site population.
            data.sites.iter_mut()
                .filter(|(_, site)| (site.seed as u64 + ctx.event.tick).is_multiple_of(SITE_CLEANUP_TICK_SKIP))
                .for_each(|(id, site)| {
                site.population.retain(|npc_id| {
                    data.npcs.get(*npc_id).is_some_and(|npc| npc.home == Some(id))
                });
            });

            // Clean up entities
            data.npcs
                .iter_mut()
                .filter(|(_, npc)| (npc.seed as u64 + ctx.event.tick).is_multiple_of(NPC_CLEANUP_TICK_SKIP))
                .for_each(|(_, npc)| npc.cleanup(&data.reports));

            // Clean up factions
            data.factions
                .iter_mut()
                .filter(|(_, faction)| (faction.seed as u64 + ctx.event.tick).is_multiple_of(FACTION_CLEANUP_TICK_SKIP))
                .for_each(|(_, faction)| faction.cleanup());

            // Clean up sites
            data.sites
                .iter_mut()
                .filter(|(_, site)| (site.seed as u64 + ctx.event.tick).is_multiple_of(SITE_CLEANUP_TICK_SKIP))
                .for_each(|(_, site)| site.cleanup(&data.reports));

            // Clean up old reports
            data.reports.cleanup(data.time_of_day);
        });

        Ok(Self)
    }
}
