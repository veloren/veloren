use tracing::info;
use vek::*;
use crate::{
    data::npc::NpcMode,
    event::OnTick,
    RtState, Rule, RuleError,
};

pub struct NpcAi;

impl Rule for NpcAi {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {

        rtstate.bind::<Self, OnTick>(|ctx| {
            for npc in ctx.state
                .data_mut()
                .npcs
                .values_mut()
            {
                // TODO: Not this
                npc.target = Some((npc.wpos + Vec3::new(ctx.event.time.sin() as f32 * 16.0, ctx.event.time.cos() as f32 * 16.0, 0.0), 1.0));
            }
        });

        Ok(Self)
    }
}
