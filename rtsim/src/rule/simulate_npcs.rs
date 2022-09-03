use crate::{data::npc::NpcMode, event::OnTick, RtState, Rule, RuleError};
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use tracing::info;
use vek::*;

pub struct SimulateNpcs;

impl Rule for SimulateNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnTick>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            for npc in data
                .npcs
                .values_mut()
                .filter(|npc| matches!(npc.mode, NpcMode::Simulated))
            {
                let body = npc.get_body();

                // Move NPCs if they have a target destination
                if let Some((target, speed_factor)) = npc.goto {
                    let diff = target.xy() - npc.wpos.xy();
                    let dist2 = diff.magnitude_squared();

                    if dist2 > 0.5f32.powi(2) {
                        npc.wpos += (diff
                            * (body.max_speed_approx() * speed_factor * ctx.event.dt
                                / dist2.sqrt())
                            .min(1.0))
                        .with_z(0.0);
                    }
                }

                // Make sure NPCs remain on the surface
                npc.wpos.z = ctx
                    .world
                    .sim()
                    .get_alt_approx(npc.wpos.xy().map(|e| e as i32))
                    .unwrap_or(0.0);
            }
        });

        Ok(Self)
    }
}
