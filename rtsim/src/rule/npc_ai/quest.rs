use super::*;
use common::comp::{Item, item::ItemBase};

/// Perform a deposit check, ensuring that the NPC has the given item and amount
/// in their inventory. If they do, the provided action is performed to
/// determine whether we should proceed. If the action chooses to proceed, then
/// we attempt to remove the items from the inventory. This may be fallible.
pub fn create_deposit<S: State, T: Action<S, bool>>(
    ctx: &mut NpcCtx,
    item: ItemResource,
    amount: f32,
    then: T,
) -> Option<impl Action<S, bool> + use<S, T>> {
    if let Some(npc_entity) = ctx.system_data.id_maps.rtsim_entity(ctx.npc_id)
        && ctx
            .system_data
            .inventories
            .lock()
            .unwrap()
            .get(npc_entity)
            .is_some_and(|inv| {
                inv.item_count(&item.to_equivalent_item_def()) >= amount.ceil() as u64
            })
    {
        Some(then.and_then(move |should_proceed: bool| {
            just(move |ctx, _| {
                if !should_proceed {
                    false
                } else if let Some(npc_entity) = ctx.system_data.id_maps.rtsim_entity(ctx.npc_id)
                    && ctx
                        .system_data
                        .inventories
                        .lock()
                        .unwrap()
                        .get_mut(npc_entity)
                        .and_then(|mut inv| {
                            inv.remove_item_amount(
                                &item.to_equivalent_item_def(),
                                amount.ceil() as u32,
                                &ctx.system_data.ability_map,
                                &ctx.system_data.msm,
                            )
                        })
                        .is_some()
                {
                    true
                } else {
                    false
                }
            })
        }))
    } else {
        None
    }
}

#[allow(clippy::result_unit_err)]
pub fn resolve_take_deposit(
    ctx: &mut NpcCtx,
    quest_id: QuestId,
    success: bool,
) -> Result<Option<(Arc<ItemDef>, u32)>, ()> {
    if let Some(outcome) = ctx
        .state
        .data()
        .quests
        .get(quest_id)
        .and_then(|q| q.resolve(ctx.npc_id, success))
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
            let item_def = item.to_equivalent_item_def();
            // Rounding down, to avoid potential precision exploits
            let amount = amount.floor() as u32;

            let mut item = Item::new_from_item_base(
                ItemBase::Simple(item_def.clone()),
                Vec::new(),
                &ctx.system_data.ability_map,
                &ctx.system_data.msm,
            );
            item.set_amount(amount)
                .expect("Item cannot be stacked that far!");
            let _ = inv.push(item);

            Ok(Some((item_def, amount)))
        } else {
            Ok(None)
        }
    } else {
        Err(())
    }
}

/// Register and create a new quest, producing its ID.
///
/// This is an action because quest creation can only happen at the end of an
/// rtsim tick (for reasons related to parallelism).
pub fn create_quest<S: State>(quest: Quest) -> impl Action<S, QuestId> {
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
                .map(|(site_id, site)| (site_id, site.wpos.as_().distance(ctx.npc.wpos.xy())))
                // Don't try to be escorted to the site we're currently in, and ensure it's a reasonable distance away
                .filter(|(site_id, dist)| Some(*site_id) != ctx.npc.current_site && (1000.0..5_000.0).contains(dist))
                // Temporarily, try to choose the same target site for 15 minutes to avoid players asking many times
                // TODO: Don't do this
                .choose(&mut ChaChaRng::from_seed([(ctx.time.0 / (60.0 * 15.0)) as u8; 32]))
            // Escort reward amount is proportional to distance
            && let escort_reward_amount = dist / 25.0
            && let Some(dst_site_name) = util::site_name(ctx, dst_site)
            && let time_limit = 1.0 + dist as f64 / 80.0
            && let Some(accept_quest) = create_deposit(ctx, ESCORT_REWARD_ITEM, escort_reward_amount, session
                    .ask_yes_no_question(Content::localized("npc-response-quest-escort-ask")
                        .with_arg("dst", dst_site_name)
                        .with_arg("coins", escort_reward_amount as u64)
                        .with_arg("mins", time_limit as u64)))
        {
            quests.push(
                accept_quest
                    .and_then(move |yes| {
                        now(move |ctx, _| {
                            if yes {
                                let quest =
                                    Quest::escort(ctx.npc_id.into(), session.target, dst_site)
                                        .with_deposit(ESCORT_REWARD_ITEM, escort_reward_amount)
                                        .with_timeout(ctx.time.add_minutes(time_limit));
                                create_quest(quest.clone())
                                    .and_then(|quest_id| {
                                        just(move |ctx, _| {
                                            ctx.controller.job = Some(Job::Quest(quest_id))
                                        })
                                    })
                                    .then(session.say_statement(Content::localized(
                                        "npc-response-quest-escort-start",
                                    )))
                                    .boxed()
                            } else {
                                session
                                    .say_statement(Content::localized(
                                        "npc-response-quest-rejected",
                                    ))
                                    .boxed()
                            }
                        })
                    })
                    .boxed(),
            );
        }

        // Kill monster quest
        const SLAY_REWARD_ITEM: ItemResource = ItemResource::Coin;
        if let Some((monster_id, monster)) = ctx
            .state
            .data()
            .npcs
            .iter()
            // Ensure the NPC is a monster
            .filter(|(_, npc)| matches!(&npc.role, Role::Monster))
            // Try to filter out monsters that are tied up in another quest (imperfect: race conditions)
            .filter(|(id, _)| ctx.state.data().quests.related_to(*id).count() == 0)
            // Filter out monsters that are too far away
            .filter(|(_, npc)| npc.wpos.xy().distance(ctx.npc.wpos.xy()) < 2500.0)
            // Find the closest
            .min_by_key(|(_, npc)| npc.wpos.xy().distance(ctx.npc.wpos.xy()) as i32)
            && let monster_pos = monster.wpos
            && let monster_body = monster.body
            && let escort_reward_amount = 200.0
            && let Some(accept_quest) = create_deposit(
                ctx,
                SLAY_REWARD_ITEM,
                escort_reward_amount,
                session.ask_yes_no_question(
                    Content::localized("npc-response-quest-slay-ask")
                        .with_arg("body", monster_body.localize_npc())
                        .with_arg("coins", escort_reward_amount as u64),
                ),
            )
        {
            quests.push(
                accept_quest
                    .and_then(move |yes| {
                        now(move |ctx, _| {
                            if yes {
                                let quest = Quest::slay(
                                    ctx.npc_id.into(),
                                    monster_id.into(),
                                    session.target,
                                )
                                .with_deposit(ESCORT_REWARD_ITEM, escort_reward_amount)
                                .with_timeout(ctx.time.add_minutes(60.0));
                                create_quest(quest.clone())
                                    .then(just(move |ctx, _| {
                                        ctx.controller.dialogue_marker(
                                            session,
                                            Marker::at(monster_pos.xy())
                                                .with_id(Actor::from(monster_id))
                                                .with_label(
                                                    Content::localized("hud-map-creature-label")
                                                        .with_arg(
                                                            "body",
                                                            monster_body.localize_npc(),
                                                        ),
                                                ),
                                        )
                                    }))
                                    .then(session.say_statement(Content::localized(
                                        "npc-response-quest-slay-start",
                                    )))
                                    .boxed()
                            } else {
                                session
                                    .say_statement(Content::localized(
                                        "npc-response-quest-rejected",
                                    ))
                                    .boxed()
                            }
                        })
                    })
                    .boxed(),
            );
        }

        if quests.is_empty() {
            session
                .say_statement(Content::localized("npc-response-quest-nothing"))
                .boxed()
        } else {
            quests.remove(ctx.rng.gen_range(0..quests.len()))
        }
    })
}

pub fn check_for_timeouts<S: State>(ctx: &mut NpcCtx) -> Option<impl Action<S> + use<S>> {
    let data = ctx.state.data();
    for quest_id in data.quests.related_to(ctx.npc_id) {
        let Some(quest) = data.quests.get(quest_id) else {
            continue;
        };
        if let Some(timeout) = quest.timeout
            // The quest has timed out...
            && ctx.time > timeout
            // ...so resolve it
            && let Ok(Some(_)) = resolve_take_deposit(ctx, quest_id, false)
        {
            // Stop any job related to the quest
            if ctx.npc.job == Some(Job::Quest(quest_id)) {
                ctx.controller.end_quest();
            }

            // If needs be, inform the quester that they failed
            match quest.kind {
                QuestKind::Escort { escorter, .. } => {
                    return Some(
                        goto_actor(escorter, 2.0)
                            .then(do_dialogue(escorter, move |session| {
                                session
                                    .say_statement(Content::localized("npc-response-quest-timeout"))
                            }))
                            .boxed(),
                    );
                },
                QuestKind::Slay { .. } => {},
            }
        }
    }
    None
}

pub fn escorted<S: State>(quest_id: QuestId, escorter: Actor, dst_site: SiteId) -> impl Action<S> {
    follow_actor(escorter, 5.0)
        .stop_if(move |ctx: &mut NpcCtx| {
            // Occasionally, tell the escoter to wait if we're lagging far behind
            if let Some(escorter_pos) = util::locate_actor(ctx, escorter)
                && ctx.npc.wpos.xy().distance_squared(escorter_pos.xy()) > 20.0f32.powi(2)
                && ctx.rng.gen_bool(ctx.dt as f64 / 30.0)
            {
                ctx.controller
                    .say(None, Content::localized("npc-speech-wait_for_me"));
            }
            // Stop if we've reached the destination site
            ctx.state
                .data()
                .sites
                .get(dst_site)
                .is_none_or(|site| site.wpos.as_().distance_squared(ctx.npc.wpos.xy()) < 150.0f32.powi(2))
        })
        .then(goto_actor(escorter, 2.0))
        .then(do_dialogue(escorter, move |session| {
            session
                .say_statement(Content::localized("npc-response-quest-escort-complete"))
                // Now that the quest has ended, resolve it and give the player the deposit
                .then(now(move |ctx, _| {
                    ctx.controller.end_quest();
                    match resolve_take_deposit(ctx, quest_id, true) {
                        Ok(deposit) => session.say_statement_with_gift(Content::localized("npc-response-quest-reward"), deposit).boxed(),
                        Err(()) => finish().boxed(),
                    }
                }))
        }))
        .stop_if(move |ctx: &mut NpcCtx| {
            // Cancel performing the quest if it's been resolved
            ctx.state
                .data()
                .quests
                .get(quest_id)
                .is_none_or(|q| q.resolution().is_some())
        })
        .map(|_, _| ())
}
