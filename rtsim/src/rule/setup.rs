use crate::{data::Site, event::OnSetup, RtState, Rule, RuleError};
use tracing::warn;

/// This rule runs at rtsim startup and broadly acts to perform some primitive
/// migration/sanitisation in order to ensure that the state of rtsim is mostly
/// sensible.
pub struct Setup;

impl Rule for Setup {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnSetup>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            // Delete rtsim sites that don't correspond to a world site
            data.sites.retain(|site_id, site| {
                if let Some((world_site_id, _)) = ctx
                    .index
                    .sites
                    .iter()
                    .find(|(_, world_site)| world_site.get_origin() == site.wpos)
                {
                    site.world_site = Some(world_site_id);
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

            for npc in data.npcs.values_mut() {
                // TODO: Consider what to do with homeless npcs.
                npc.home = npc.home.filter(|home| data.sites.contains_key(*home));
            }

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
                    ));
                }
            }

            // TODO: Reassign sites for NPCs if they don't have one
        });

        Ok(Self)
    }
}
