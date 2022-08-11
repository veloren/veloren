use tracing::info;
use crate::{
    data::npc::NpcLoc,
    event::OnTick,
    RtState, Rule, RuleError,
};

pub struct SimulateNpcs;

impl Rule for SimulateNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {

        rtstate.bind::<Self, OnTick>(|ctx| {
            for (_, npc) in ctx.state.data_mut().npcs.iter_mut() {
                npc.tick_wpos(match npc.loc {
                    NpcLoc::Wild { wpos } => wpos,
                    NpcLoc::Site { site, wpos } => wpos,
                    NpcLoc::Travelling { a, b, frac } => todo!(),
                });
            }
        });

        Ok(Self)
    }
}
