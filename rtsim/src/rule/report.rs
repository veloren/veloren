use crate::{
    data::{report::ReportKind, Report},
    event::{EventCtx, OnDeath, OnTheft},
    RtState, Rule, RuleError,
};
use common::rtsim::NpcInput;

pub struct ReportEvents;

impl Rule for ReportEvents {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnDeath>(on_death);
        rtstate.bind::<Self, OnTheft>(on_theft);

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
                at_tod: data.time_of_day,
            });

            // TODO: Don't push report to NPC inboxes, have a dedicated data structure that
            // tracks reports by chunks and then have NPCs decide to query this
            // data structure in their own time.
            for npc_id in nearby {
                if let Some(npc) = data.npcs.get_mut(npc_id) {
                    npc.inbox.push_back(NpcInput::Report(report));
                }
            }
        }
    }
}

fn on_theft(ctx: EventCtx<ReportEvents, OnTheft>) {
    let data = &mut *ctx.state.data_mut();

    let nearby = data
        .npcs
        .nearby(None, ctx.event.wpos.as_(), 24.0)
        .filter_map(|actor| actor.npc())
        .collect::<Vec<_>>();

    if !nearby.is_empty() {
        let report = data.reports.create(Report {
            kind: ReportKind::Theft {
                theif: ctx.event.actor,
                site: ctx.event.site,
                sprite: ctx.event.sprite,
            },
            at_tod: data.time_of_day,
        });

        for npc_id in nearby {
            if let Some(npc) = data.npcs.get_mut(npc_id) {
                npc.inbox.push_back(NpcInput::Report(report));
            }
        }
    }
}
