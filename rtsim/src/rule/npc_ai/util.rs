use super::*;

pub fn site_name(ctx: &NpcCtx, site_id: impl Into<Option<SiteId>>) -> Option<String> {
    let world_site = ctx.state.data().sites.get(site_id.into()?)?.world_site?;
    Some(ctx.index.sites.get(world_site).name().to_string())
}

pub fn locate_actor(ctx: &NpcCtx, actor: Actor) -> Option<Vec3<f32>> {
    match actor {
        Actor::Npc(npc_id) => ctx.state.data().npcs.get(npc_id).map(|npc| npc.wpos),
        Actor::Character(character_id) => ctx
            .system_data
            .id_maps
            .character_entity(character_id)
            .and_then(|c| ctx.system_data.positions.get(c))
            .map(|p| p.0),
    }
}

pub fn actor_exists(ctx: &NpcCtx, actor: Actor) -> bool {
    match actor {
        Actor::Npc(npc_id) => ctx.state.data().npcs.contains_key(npc_id),
        Actor::Character(character_id) => ctx
            .system_data
            .id_maps
            .character_entity(character_id)
            .is_some(),
    }
}
