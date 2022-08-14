use crate::{data::npc::NpcMode, event::OnTick, RtState, Rule, RuleError};
use rand::seq::IteratorRandom;
use tracing::info;
use vek::*;
use world::site::SiteKind;

pub struct NpcAi;

impl Rule for NpcAi {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnTick>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            for npc in data.npcs.values_mut() {
                if let Some(home_id) = npc
                    .home
                    .and_then(|site_id| data.sites.get(site_id)?.world_site)
                {
                    if let Some((target, _)) = npc.target {
                        if target.distance_squared(npc.wpos) < 1.0 {
                            npc.target = None;
                        }
                    } else {
                        match &ctx.index.sites.get(home_id).kind {
                            SiteKind::Refactor(site)
                            | SiteKind::CliffTown(site)
                            | SiteKind::DesertCity(site) => {
                                let tile = site.wpos_tile_pos(npc.wpos.xy().as_());

                                let mut rng = rand::thread_rng();
                                let cardinals = [
                                    Vec2::unit_x(),
                                    Vec2::unit_y(),
                                    -Vec2::unit_x(),
                                    -Vec2::unit_y(),
                                ];
                                let next_tile = cardinals
                                    .iter()
                                    .map(|c| tile + *c)
                                    .filter(|tile| site.tiles.get(*tile).is_road()).choose(&mut rng).unwrap_or(tile);

                                let wpos =
                                    site.tile_center_wpos(next_tile).as_().with_z(npc.wpos.z);

                                npc.target = Some((wpos, 1.0));
                            },
                            _ => {
                                // No brain T_T
                            },
                        }
                    }
                } else {
                    // TODO: Don't make homeless people walk around in circles
                    npc.target = Some((
                        npc.wpos
                            + Vec3::new(
                                ctx.event.time.sin() as f32 * 16.0,
                                ctx.event.time.cos() as f32 * 16.0,
                                0.0,
                            ),
                        1.0,
                    ));
                }
            }
        });

        Ok(Self)
    }
}
