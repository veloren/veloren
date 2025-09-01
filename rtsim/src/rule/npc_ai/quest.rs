use super::*;
use common::comp::{Item, item::ItemBase};

pub fn escorted<S: State>(quest_id: QuestId, escorter: Actor, tgt_site: SiteId) -> impl Action<S> {
    follow_actor(escorter, 5.0)
        .stop_if(move |ctx: &mut NpcCtx| ctx.npc.current_site == Some(tgt_site))
        .then(now(move |ctx, _| {
            ctx.controller.end_quest();
            // Now that the quest has ended, resolve it...
            if let Some(outcome) = ctx
                .state
                .data()
                .quests
                .get(quest_id)
                .and_then(|q| q.resolve(ctx.npc_id, true))
            {
                // ...take the deposit back into our own inventory...
                if let Some((item_def, amount)) = &outcome.deposit
                    && let Some(npc_entity) = ctx.system_data.id_maps.rtsim_entity(ctx.npc_id)
                    && let Some(mut inv) = ctx
                        .system_data
                        .inventories
                        .lock()
                        .unwrap()
                        .get_mut(npc_entity)
                {
                    let mut item = Item::new_from_item_base(
                        ItemBase::Simple(item_def.clone()),
                        Vec::new(),
                        &ctx.system_data.ability_map,
                        &ctx.system_data.msm,
                    );
                    item.set_amount(*amount);
                    let _ = inv.push(item);
                }

                // ...and then give it to the player!
                goto_actor(escorter, 2.0)
                    .then(do_dialogue(escorter, move |session| {
                        session
                            .say_statement(Content::localized("dialogue-quest-escort-complete"))
                            .then(session.say_statement_with_gift(
                                Content::localized("dialogue-quest-reward"),
                                outcome.deposit.clone(),
                            ))
                    }))
                    .boxed()
            } else {
                // Following finished but quest was already resolved?!
                finish().boxed()
            }
        }))
}
