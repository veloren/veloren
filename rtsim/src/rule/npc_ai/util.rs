use super::*;

pub fn site_name(ctx: &NpcCtx, site_id: impl Into<Option<SiteId>>) -> Option<String> {
    let world_site = ctx.state.data().sites.get(site_id.into()?)?.world_site?;
    Some(ctx.index.sites.get(world_site).name().to_string())
}
