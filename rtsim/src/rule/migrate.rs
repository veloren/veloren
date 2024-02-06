use crate::{data::Site, event::OnSetup, RtState, Rule, RuleError};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use tracing::warn;
use world::site::SiteKind;

/// This rule runs at rtsim startup and broadly acts to perform some primitive
/// migration/sanitisation in order to ensure that the state of rtsim is mostly
/// sensible.
pub struct Migrate;

impl Rule for Migrate {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnSetup>(|ctx| {
            let data = &mut *ctx.state.data_mut();

            let mut rng = ChaChaRng::from_seed(thread_rng().gen::<[u8; 32]>());

            // Delete rtsim sites that don't correspond to a world site
            data.sites.sites.retain(|site_id, site| {
                if let Some((world_site_id, _)) = ctx
                    .index
                    .sites
                    .iter()
                    .find(|(_, world_site)| world_site.get_origin() == site.wpos)
                {
                    site.world_site = Some(world_site_id);
                    data.sites.world_site_map.insert(world_site_id, site_id);
                    true
                } else {
                    warn!(
                        "{:?} is no longer valid because the site it was derived from no longer \
                         exists. It will now be deleted.",
                        site_id
                    );
                    false
                }
            });

            // Generate rtsim sites for world sites that don't have a corresponding rtsim
            // site yet
            for (world_site_id, _) in ctx.index.sites.iter() {
                if !data.sites.values().any(|site| {
                    site.world_site
                        .expect("Rtsim site not assigned to world site")
                        == world_site_id
                }) {
                    warn!(
                        "{:?} is new and does not have a corresponding rtsim site. One will now \
                         be generated afresh.",
                        world_site_id
                    );
                    data.sites.create(Site::generate(
                        world_site_id,
                        ctx.world,
                        ctx.index,
                        &[],
                        &data.factions,
                        &mut rng,
                    ));
                }
            }

            // Reassign NPCs to sites if their old one was deleted. If they were already
            // homeless, no need to do anything.
            for npc in data.npcs.values_mut() {
                if let Some(home) = npc.home
                    && !data.sites.contains_key(home)
                {
                    // Choose the closest habitable site as the new home for the NPC
                    npc.home = data
                        .sites
                        .sites
                        .iter()
                        .filter(|(_, site)| {
                            // TODO: This is a bit silly, but needs to wait on the removal of site1
                            site.world_site.map_or(false, |ws| {
                                matches!(
                                    &ctx.index.sites.get(ws).kind,
                                    SiteKind::Refactor(_)
                                        | SiteKind::CliffTown(_)
                                        | SiteKind::SavannahPit(_)
                                        | SiteKind::CoastalTown(_)
                                        | SiteKind::DesertCity(_)
                                )
                            })
                        })
                        .min_by_key(|(_, site)| {
                            site.wpos.as_().distance_squared(npc.wpos.xy()) as i32
                        })
                        .map(|(site_id, _)| site_id);
                }
            }
        });

        Ok(Self)
    }
}
