use super::*;

pub fn escorted<S: State>(escorter: Actor, tgt_site: SiteId) -> impl Action<S> {
    follow_actor(escorter, 5.0)
        .stop_if(move |ctx: &mut NpcCtx| ctx.npc.current_site == Some(tgt_site))
        .then(
            goto_actor(escorter, 2.0).then(do_dialogue(escorter, |session| {
                session.say_statement(Content::localized("dialogue-quest-escort-complete"))
            })),
        )
        .then(just(|ctx, _| ctx.controller.end_quest()))
}
