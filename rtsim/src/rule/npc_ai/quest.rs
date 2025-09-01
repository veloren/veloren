use super::*;
use common::comp::{Item, item::ItemBase};

/// Register and create a new quest, producing its ID.
///
/// This is an action because quest creation can only happen at the end of an
/// rtsim tick (for reasons related to parallelism).
fn create_quest<S: State>(quest: Quest) -> impl Action<S, QuestId> {
    just(move |ctx, _| {
        let quest_id = ctx.state.data().quests.register();
        ctx.controller
            .quests_to_create
            .push((quest_id, quest.clone()));
        quest_id
    })
}

pub fn quest_request<S: State>(session: DialogueSession) -> impl Action<S> {
    now(move |ctx, _| {
        let mut quests = Vec::new();

        // Escort quest.
        const ESCORT_REWARD_ITEM: ItemResource = ItemResource::Coin;
        // Escortable NPCs must have no existing job
        if ctx.npc.job.is_none()
            // They must be a merchant
            && matches!(ctx.npc.profession(), Some(Profession::Merchant))
            // Choose an appropriate target site
            && let Some((dst_site, dist)) = ctx
                .state
                .data()
                .sites
                .iter()
                // Find the distance to the site
                .map(|(site_id, site)| (site_id, site.wpos.as_().distance(ctx.npc.wpos.xy()) as u32))
                // Don't try to be escorted to the site we're currently in, and ensure it's a reasonable distance away
                .filter(|(site_id, dist)| Some(*site_id) != ctx.npc.current_site && (1000..5_000).contains(dist))
                .choose(&mut ctx.rng)
            // Escort reward amount is proportional to distance
            && let escort_reward_amount = dist / 25
            && let Some(dst_site_name) = util::site_name(ctx, dst_site)
            // Ensure the NPC has the reward in their inventory
            && let Some(npc_entity) = ctx.system_data.id_maps.rtsim_entity(ctx.npc_id)
            && ctx.system_data.inventories.lock().unwrap()
                .get(npc_entity)
                .is_some_and(|inv| inv.item_count(&ESCORT_REWARD_ITEM.to_equivalent_item_def()) >= escort_reward_amount as u64)
        {
            let time_limit = 1 + dist / 200;
            quests.push(
                session
                    .ask_yes_no_question(Content::localized("dialogue-quest-escort-ask")
                        .with_arg("dst", dst_site_name)
                        .with_arg("coins", escort_reward_amount as u64)
                        .with_arg("mins", time_limit as u64))
                    .and_then(move |yes| now(move |ctx, _| {
                        if yes
                            // Remove the reward from the NPC's inventory
                            && let Some(npc_entity) = ctx.system_data.id_maps.rtsim_entity(ctx.npc_id)
                            && let Some(deposit) = ctx.system_data.inventories.lock().unwrap().get_mut(npc_entity)
                                .and_then(|mut inv| inv.remove_item_amount(
                                    &ESCORT_REWARD_ITEM.to_equivalent_item_def(),
                                    escort_reward_amount,
                                    &ctx.system_data.ability_map,
                                    &ctx.system_data.msm,
                                ))
                        {
                            let quest = Quest::escort(ctx.npc_id.into(), session.target, dst_site)
                                .with_deposit(ItemResource::Coin, escort_reward_amount as f32)
                                .with_timeout(ctx.time.add_minutes(time_limit as f64));
                            create_quest(quest.clone())
                                .and_then(|quest_id| {
                                    just(move |ctx, _| {
                                        ctx.controller.job = Some(Job::Quest(quest_id))
                                    })
                                })
                                .then(session.say_statement(Content::localized(
                                    "dialogue-quest-escort-start",
                                )))
                                .boxed()
                        } else {
                            session
                                .say_statement(Content::localized("dialogue-quest-rejected"))
                                .boxed()
                        }
                    }))
                    .boxed(),
            );
        }

        if quests.is_empty() {
            session
                .say_statement(Content::localized("dialogue-quest-nothing"))
                .boxed()
        } else {
            quests.remove(ctx.rng.gen_range(0..quests.len()))
        }
    })
}

pub fn escorted<S: State>(quest_id: QuestId, escorter: Actor, dst_site: SiteId) -> impl Action<S> {
    follow_actor(escorter, 5.0)
        // Stop if we've reached the destination site
        .stop_if(move |ctx: &mut NpcCtx| ctx.state.data().sites
            .get(dst_site)
            .map_or(true, |site| site.wpos.as_().distance(ctx.npc.wpos.xy()) < 64.0))
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
                if let Some((item, amount)) = &outcome.deposit
                    && let Some(npc_entity) = ctx.system_data.id_maps.rtsim_entity(ctx.npc_id)
                    && let Some(mut inv) = ctx
                        .system_data
                        .inventories
                        .lock()
                        .unwrap()
                        .get_mut(npc_entity)
                {
                    let mut item = Item::new_from_item_base(
                        ItemBase::Simple(item.to_equivalent_item_def()),
                        Vec::new(),
                        &ctx.system_data.ability_map,
                        &ctx.system_data.msm,
                    );
                    // Rounding down, to avoid potential precision exploits
                    item.set_amount(amount.floor() as u32);
                    let _ = inv.push(item);
                }

                // ...and then give it to the player!
                goto_actor(escorter, 2.0)
                    .then(do_dialogue(escorter, move |session| {
                        session
                            .say_statement(Content::localized("dialogue-quest-escort-complete"))
                            .then(session.say_statement_with_gift(
                                Content::localized("dialogue-quest-reward"),
                                outcome.deposit.map(|(item, amount)| {
                                    (item.to_equivalent_item_def(), amount.floor() as u32)
                                }),
                            ))
                    }))
                    .boxed()
            } else {
                // Following finished but quest was already resolved?!
                finish().boxed()
            }
        }))
        .stop_if(move |ctx: &mut NpcCtx| {
            if let Some(timeout) = ctx.state.data().quests.get(quest_id).and_then(|q| q.timeout) {
                ctx.time > timeout
            } else {
                false
            }
        })
        .and_then(move |r: Option<()>| if r.is_none() {
            goto_actor(escorter, 2.0)
                .then(do_dialogue(escorter, move |session| session.say_statement(Content::localized("dialogue-quest-timeout"))))
                .boxed()
        } else {
            finish().boxed()
        })
}
