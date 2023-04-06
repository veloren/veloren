use crate::{
    data::{report::ReportKind, Report},
    event::{EventCtx, OnDeath},
    RtState, Rule, RuleError,
};

pub struct ReportEvents;

impl Rule for ReportEvents {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnDeath>(on_death);

        Ok(Self)
    }
}

fn on_death(ctx: EventCtx<ReportEvents, OnDeath>) {
    let data = &mut *ctx.state.data_mut();

    if let Some(wpos) = ctx.event.wpos {
        let nearby = data
            .npcs
            .nearby(None, wpos, 32.0)
            .filter_map(|actor| actor.npc())
            .collect::<Vec<_>>();

        if !nearby.is_empty() {
            let report = data.reports.create(Report {
                kind: ReportKind::Death {
                    actor: ctx.event.actor,
                    killer: ctx.event.killer,
                },
                at: data.time_of_day,
            });

            for npc_id in nearby {
                if let Some(npc) = data.npcs.get_mut(npc_id) {
                    npc.inbox.push_back(report);
                }
            }
        }
    }
}
