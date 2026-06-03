use crate::data::quest::{
    COURIER_QUEST_VARIANTS, CourierQuest, CourierQuestInstance, Payload, Recipient,
};

use super::*;
use common::{
    comp::{Item, item::ItemBase},
    rtsim::NpcId,
    spot::Spot,
};

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
        .data
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

/// Checks if a courier quest can be completed based on inventory and entity
/// presence.
///
/// The inventory check/consume operation is atomic. All inventory items'
/// presence are verified first, then the items are subsequently removed in the
/// same transaction/lock.
///
/// That being said, if `read_only` is true, the inventory will not be modified,
/// only checked.
///
/// This should support checking for completion regardless of if it's being
/// completed by a player or an rtsim NPC.
pub fn finalize_courier_task(ctx: &mut NpcCtx, quest_id: QuestId, read_only: bool) -> bool {
    fn required_count(raw: f32) -> u32 {
        debug_assert!(
            raw.is_finite() && raw >= 0.0,
            "courier quest required amount must be finite and non-negative, got {raw}",
        );
        raw.round() as u32
    }

    if let Some(quest) = ctx.data.quests.get(quest_id)
        && let QuestKind::Courier { instance } = &quest.kind
        && let Some(entity) = match instance.messenger {
            Actor::Character(character_id) => {
                ctx.system_data.id_maps.character_entity(character_id)
            },
            Actor::Npc(npc_id) => ctx.system_data.id_maps.rtsim_entity(npc_id),
        }
        && let Ok(mut inventories) = ctx.system_data.inventories.lock()
        && let Some(mut inv) = inventories.get_mut(entity)
        && let Ok(required_items) = instance.get_required_items()
        && required_items.iter().all(|(item_def, amount)| {
            inv.item_count(item_def) >= u64::from(required_count(*amount))
        })
        && (read_only
            || required_items.iter().all(|(item_def, amount)| {
                inv.remove_item_amount(
                    item_def,
                    required_count(*amount),
                    &ctx.system_data.ability_map,
                    &ctx.system_data.msm,
                )
                .is_some()
            }))
    {
        true
    } else {
        false
    }
}

/// Register and create a new quest, producing its ID.
///
/// This is an action because quest creation can only happen at the end of an
/// rtsim tick (for reasons related to parallelism).
pub fn create_quest<S: State>(quest: Quest) -> impl Action<S, QuestId> {
    just(move |ctx, _| {
        let quest_id = ctx.data.quests.register();
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
            && let Some((dst_site_id, dst_site, dist)) = ctx.data
                .sites
                .iter()
                // Find the distance to the site
                .map(|(site_id, site)| (site_id, site, site.wpos.as_().distance(ctx.npc.wpos.xy())))
                // Don't try to be escorted to the site we're currently in, and ensure it's a reasonable distance away
                .filter(|(site_id, _, dist)| Some(*site_id) != ctx.npc.current_site && (1000.0..5_000.0).contains(dist))
                // Temporarily, try to choose the same target site for 15 minutes to avoid players asking many times
                // TODO: Don't do this
                .choose(&mut ChaChaRng::from_seed([(ctx.time.0 / (60.0 * 15.0)) as u8; 32]))
            // Escort reward amount is proportional to distance
            && let escort_reward_amount = dist / 25.0
            && let Some(dst_site_name) = util::site_name(ctx, dst_site_id)
            && let time_limit = 1.0 + dist as f64 / 80.0
            && let Some(accept_quest) = create_deposit(ctx, ESCORT_REWARD_ITEM, escort_reward_amount, session
                    .ask_yes_no_question(Content::localized("npc-response-quest-escort-ask")
                        .with_arg("dst", dst_site_name.clone())
                        .with_arg("coins", escort_reward_amount as u64)
                        .with_arg("mins", time_limit as u64)))
        {
            let dst_wpos = dst_site.wpos.as_();
            quests.push(
                accept_quest
                    .and_then(move |yes| {
                        now(move |ctx, _| {
                            if yes {
                                let quest =
                                    Quest::escort(ctx.npc_id.into(), session.target, dst_site_id)
                                        .with_deposit(ESCORT_REWARD_ITEM, escort_reward_amount)
                                        .with_timeout(ctx.time.add_minutes(time_limit));
                                create_quest(quest.clone())
                                    .and_then(move |quest_id| {
                                        now(move |ctx, _| {
                                            ctx.controller.job = Some(Job::Quest(quest_id));
                                            session.give_marker(
                                                Marker::at(dst_wpos)
                                                    .with_id(quest_id)
                                                    .with_label(
                                                        Content::localized("hud-map-escort-label")
                                                            .with_arg(
                                                                "name",
                                                                ctx.npc.get_name().unwrap_or_else(
                                                                    || "<unknown>".to_string(),
                                                                ),
                                                            )
                                                            .with_arg(
                                                                "place",
                                                                dst_site_name.clone(),
                                                            ),
                                                    )
                                                    .with_quest_flag(true),
                                            )
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
        if let Some((monster_id, monster)) = ctx.data
            .npcs
            .iter()
            // Ensure the NPC is a monster
            .filter(|(_, npc)| matches!(&npc.role, Role::Monster))
            // Try to filter out monsters that are tied up in another quest (imperfect: race conditions)
            .filter(|(id, _)| ctx.data.quests.related_to(*id).count() == 0)
            // Filter out monsters that are too far away
            .filter(|(_, npc)| npc.wpos.xy().distance(ctx.npc.wpos.xy()) < 2500.0)
            // Find the closest
            .min_by_key(|(_, npc)| npc.wpos.xy().distance_squared(ctx.npc.wpos.xy()) as i64)
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
                                    .then(
                                        session.give_marker(
                                            Marker::at(monster_pos.xy())
                                                .with_id(Actor::from(monster_id))
                                                .with_label(
                                                    Content::localized("hud-map-creature-label")
                                                        .with_arg(
                                                            "body",
                                                            monster_body.localize_npc(),
                                                        ),
                                                )
                                                .with_quest_flag(true),
                                        ),
                                    )
                                    .then(session.say_statement(Content::localized(
                                        "npc-response-quest-slay-start",
                                    )))
                                    .then(session.say_statement(Content::localized(
                                        "npc-response-quest-slay-start_2",
                                    )))
                                    .then(session.say_statement(Content::localized(
                                        "npc-response-quest-slay-start_3",
                                    )))
                                    .then(session.say_statement(Content::localized(
                                        "npc-response-quest-slay-start_4",
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

        const COURIER_REWARD_ITEM: ItemResource = ItemResource::Coin;
        if let Some(courier_quest) = roll_courier_quest(ctx, session.target)
            && let Some(quest_tgt) = match courier_quest.target_actor {
                Actor::Npc(tgt_npc_id) => ctx.data.npcs.npcs.get(tgt_npc_id),
                // Courier quests between players is not supported right now,
                // but here's the scaffolding for it
                Actor::Character(_) => None,
            }
            && let Some(tgt_site_name) = courier_quest
                .target_site
                .and_then(|tgt_site_id| ctx.data.sites.get(tgt_site_id))
                .and_then(|queried_site| queried_site.world_site)
                .and_then(|queried_site_world_id| ctx.index.sites.get(queried_site_world_id).name())
        {
            let quest_tgt_actor = courier_quest.target_actor;
            let (start_stmt, start_question) = courier_quest.get_start_dialogue(
                quest_tgt
                    .get_name()
                    .unwrap_or_else(|| "<unknown>".to_string())
                    .as_str(),
                tgt_site_name,
            );
            let proposed_quest = create_deposit(
                ctx,
                COURIER_REWARD_ITEM,
                courier_quest.get_reward(),
                session
                    .say_statement(start_stmt)
                    .then(session.ask_yes_no_question(start_question)),
            );

            if let Some(accept_quest) = proposed_quest {
                // define a few values before entering closures
                let tgt_name = quest_tgt
                    .get_name()
                    .unwrap_or_else(|| "<unknown>".to_string());
                let tgt_name_marker = tgt_name.clone();
                let tgt_actor_wpos = quest_tgt.wpos.xy();
                let tgt_actor = quest_tgt_actor;
                let quest_exp = ctx.time.add_minutes(180.0);

                let quest_offer = accept_quest
                    .and_then(move |yes| {
                        now(move |_ctx, _| {
                            if yes {
                                let quest = Quest::courier(tgt_actor, courier_quest)
                                    .with_deposit(COURIER_REWARD_ITEM, courier_quest.get_reward())
                                    .with_timeout(quest_exp);
                                create_quest(quest)
                                    .and_then(move |quest_id| {
                                        now(move |_ctx, _| {
                                            if let Some(chunk_pos) = courier_quest.spot {
                                                let chunk_wpos = chunk_pos.cpos_to_wpos();
                                                // provide a map marker that points to the
                                                // nearest spot
                                                session.give_marker(
                                                    Marker::at(Vec2::new(
                                                        chunk_wpos.x as f32,
                                                        chunk_wpos.y as f32,
                                                    ))
                                                    .with_id(quest_id)
                                                    .with_label(
                                                        courier_quest
                                                            .get_spot_map_label(tgt_name.as_str()),
                                                    )
                                                    .with_quest_flag(true),
                                                )
                                            } else {
                                                // provide a map marker that points to the
                                                // courier target
                                                session.give_marker(
                                                    Marker::at(tgt_actor_wpos)
                                                        .with_id(tgt_actor)
                                                        .with_label(
                                                            Content::localized(
                                                                "hud-map-character-label",
                                                            )
                                                            .with_arg("name", tgt_name.as_str()),
                                                        )
                                                        .with_kind(MarkerKind::Character),
                                                )
                                            }
                                        })
                                    })
                                    .then(
                                        // provide a map marker that points to the courier
                                        // target (note: this does it twice if there is a spot,
                                        // only because we have to do something in the previous
                                        // .and_then() statement to satisfy type symmetry)
                                        session.give_marker(
                                            Marker::at(tgt_actor_wpos)
                                                .with_id(tgt_actor)
                                                .with_label(
                                                    Content::localized("hud-map-character-label")
                                                        .with_arg("name", tgt_name_marker),
                                                )
                                                .with_kind(MarkerKind::Character),
                                        ),
                                    )
                                    .then(session.say_statement(Content::localized(
                                        "npc-response-quest-courier-start",
                                    )))
                                    .then(session.say_statement(Content::localized(
                                        "npc-response-quest-courier-start_2",
                                    )))
                                    .then(session.say_statement(Content::localized(
                                        "npc-response-quest-courier-start_3",
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
                    .boxed();

                quests.push(quest_offer);
            }
        }

        if quests.is_empty() {
            session
                .say_statement(Content::localized("npc-response-quest-nothing"))
                .boxed()
        } else {
            quests.remove(ctx.rng.random_range(0..quests.len()))
        }
    })
}

pub fn check_for_timeouts<S: State>(ctx: &mut NpcCtx) -> Option<impl Action<S> + use<S>> {
    for quest_id in ctx.data.quests.related_to(ctx.npc_id) {
        let Some(quest) = ctx.data.quests.get(quest_id) else {
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
                QuestKind::Courier { .. } => {},
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
                && ctx.rng.random_bool(ctx.dt as f64 / 30.0)
            {
                ctx.controller
                    .say(None, Content::localized("npc-speech-wait_for_me"));
            }
            // Stop if we've reached the destination site
            ctx.data
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
            ctx.data
                .quests
                .get(quest_id)
                .is_none_or(|q| q.resolution().is_some())
        })
        .map(|_, _| ())
}

/// Finds the nearest chunk position that contains the appropriate kind of spot
/// for this courier quest variant. For example, if you have a Gnarling Carving
/// quest, this will search nearby for the nearest Gnarling Totem spot and
/// return the chunk position (not the world position, you'll need to convert it
/// to `wpos`).
///
/// The `target_chunk` needs to be predetermined in order to satisfy compiler
/// checks.
pub fn get_nearest_spot(
    ctx: &mut NpcCtx,
    quest: CourierQuest,
    target_chunk: Vec2<i32>,
) -> Option<Vec2<i32>> {
    match quest.payload() {
        // These do not have spots
        None | Some(Payload::LegoomLeaf) => None,
        // Add more here later!
        Some(Payload::GnarlingCarving) => ctx
            .world
            .sim()
            .get_nearest_spot(target_chunk, |spot| matches!(spot, Spot::GnarlingTotem)),
    }
}

/// This file only contains an implementation for quest interactions. Make sure
/// to look for other implementations.
impl CourierQuestInstance {
    /// Returns a list of all items that are required for completing this
    /// courier quest.
    pub fn get_required_items(self) -> Result<Vec<(Arc<ItemDef>, f32)>, common::assets::Error> {
        match self.kind.payload() {
            None => Ok(vec![]),
            Some(Payload::GnarlingCarving) => Ok(vec![(
                Arc::<ItemDef>::load_cloned("common.items.quest.gnarling_carving")?,
                1.0_f32,
            )]),
            Some(Payload::LegoomLeaf) => Ok(vec![(
                Arc::<ItemDef>::load_cloned("common.items.quest.legoom_leaf")?,
                1.0_f32,
            )]),
        }
    }

    /// Returns the number of coins that the quest arbiter must pay upon courier
    /// quest completion. Note that in some cases the arbiter is not the
    /// person that paid the quest deposit.
    pub fn get_reward(self) -> f32 {
        match self.kind {
            CourierQuest::Message => 150.0,
            CourierQuest::Deliver {
                payload: Payload::GnarlingCarving,
                recipient: Recipient::Other,
            } => 275.0,
            CourierQuest::Deliver {
                payload: Payload::GnarlingCarving,
                recipient: Recipient::Giver,
            } => 200.0,
            CourierQuest::Deliver {
                payload: Payload::LegoomLeaf,
                recipient: Recipient::Other,
            } => 240.0,
            CourierQuest::Deliver {
                payload: Payload::LegoomLeaf,
                recipient: Recipient::Giver,
            } => 100.0,
        }
    }

    /// Retrieves the i18n content that will be shown on the map when hovering
    /// over the courier quest's map marker.
    pub fn get_spot_map_label(self, npc_name: &str) -> Content {
        Content::localized(match self.kind.payload() {
            Some(Payload::GnarlingCarving) => "hud-map-spot-gnarling-carving-label",
            // These shouldn't be encountered since they don't have spot requirements:
            None | Some(Payload::LegoomLeaf) => "hud-map-spot-unspecified",
        })
        .with_arg("name", npc_name)
    }

    /// "You don't have enough items on you to complete this quest."
    pub fn lacks_items(self) -> Content {
        Content::localized(match self.kind.payload() {
            Some(Payload::GnarlingCarving) => {
                "npc-response-quest-courier-gnarling-carving-insufficient-items"
            },
            Some(Payload::LegoomLeaf) => {
                "npc-response-quest-courier-legoom-leaf-insufficient-items"
            },
            // For quests that do not require items, use this arm.
            None => "npc-response-quest-courier-generic-insufficient-items",
        })
    }

    /// Assembles the dialogue question and response when asking what items
    /// are needed in order to complete an active courier quest.
    ///
    /// "What am I supposed to be getting for you/target again?"
    /// "You need X, Y, and Z to complete this courier quest."
    pub fn what_items_needed(self, is_target_npc: bool, npc_name: &str) -> (Content, Content) {
        (
            Content::localized(match self.kind {
                CourierQuest::Deliver {
                    recipient: Recipient::Other,
                    ..
                } => {
                    if is_target_npc {
                        "dialogue-question-quest-courier-what-target"
                    } else {
                        "dialogue-question-quest-courier-what"
                    }
                },
                CourierQuest::Deliver {
                    recipient: Recipient::Giver,
                    ..
                } => "dialogue-question-quest-fetch-what",
                CourierQuest::Message => {
                    if is_target_npc {
                        "dialogue-question-quest-messenger-what-target"
                    } else {
                        "dialogue-question-quest-messenger-what"
                    }
                },
            })
            .with_arg("name", npc_name),
            Content::localized(match self.kind.payload() {
                Some(Payload::GnarlingCarving) => {
                    "npc-response-quest-courier-gnarling-carving-what-is-needed"
                },
                Some(Payload::LegoomLeaf) => {
                    "npc-response-quest-courier-legoom-leaf-what-is-needed"
                },
                None => {
                    if is_target_npc {
                        "npc-response-quest-messenger-what-is-needed-target"
                    } else {
                        "npc-response-quest-messenger-what-is-needed"
                    }
                },
            })
            .with_arg("name", npc_name),
        )
    }

    /// Retrieves the i18n content for the name of the spot, or a generic
    /// response if the courier quest variant does not need a spot.
    pub fn get_spot_name(self) -> Content {
        Content::localized(match self.kind.payload() {
            Some(Payload::GnarlingCarving) => "spot-name-gnarling-totem",
            None | Some(Payload::LegoomLeaf) => "spot-name-unspecified",
        })
    }

    /// Returns the i18n content for the initial courier quest
    /// statement/preamble that an NPC will say, as well as the subsequent
    /// yes/no question that they ask that allows starting the quest.
    ///
    /// Note that not every quest uses the target npc name or the target site
    /// name.
    pub fn get_start_dialogue(
        self,
        tgt_npc_name_str: &str,
        tgt_site_name: &str,
    ) -> (Content, Content) {
        const COURIER_GNARLING_CARVING_START_STMT: &str =
            "npc-response-quest-courier-gnarling-carving";
        const COURIER_LEGOOM_LEAF_START_STMT: &str = "npc-response-quest-courier-legoom-leaf";
        const MESSENGER_SEND_WORD_START_STMT: &str = "npc-response-quest-messenger-send-word";

        match self.kind {
            CourierQuest::Deliver {
                payload: Payload::GnarlingCarving,
                recipient: Recipient::Other,
            } => (
                Content::localized(COURIER_GNARLING_CARVING_START_STMT),
                Content::localized("npc-response-quest-spot-courier-ask")
                    .with_arg("spot", self.get_spot_name())
                    .with_arg("coins", self.get_reward() as u64)
                    .with_arg("name", tgt_npc_name_str)
                    .with_arg("site", tgt_site_name),
            ),
            CourierQuest::Deliver {
                payload: Payload::GnarlingCarving,
                recipient: Recipient::Giver,
            } => (
                Content::localized(COURIER_GNARLING_CARVING_START_STMT),
                Content::localized("npc-response-quest-spot-fetch-ask")
                    .with_arg("spot", self.get_spot_name())
                    .with_arg("coins", self.get_reward() as u64),
            ),
            CourierQuest::Deliver {
                payload: Payload::LegoomLeaf,
                recipient: Recipient::Other,
            } => (
                Content::localized(COURIER_LEGOOM_LEAF_START_STMT),
                Content::localized("npc-response-quest-courier-ask")
                    .with_arg("coins", self.get_reward() as u64)
                    .with_arg("name", tgt_npc_name_str)
                    .with_arg("site", tgt_site_name),
            ),
            CourierQuest::Deliver {
                payload: Payload::LegoomLeaf,
                recipient: Recipient::Giver,
            } => (
                Content::localized(COURIER_LEGOOM_LEAF_START_STMT),
                Content::localized("npc-response-quest-fetch-ask")
                    .with_arg("coins", self.get_reward() as u64),
            ),
            CourierQuest::Message => (
                Content::localized(MESSENGER_SEND_WORD_START_STMT),
                Content::localized("npc-response-quest-messenger-ask")
                    .with_arg("coins", self.get_reward() as u64)
                    .with_arg("name", tgt_npc_name_str)
                    .with_arg("site", tgt_site_name),
            ),
        }
    }

    /// "Where is my target again?"
    /// Map gets marked with a marker, and the NPC responds with their location.
    pub fn get_dialogue_where_target(
        self,
        npc_name: &str,
        at: Vec2<f32>,
        target: Actor,
    ) -> (Content, Marker, Content) {
        (
            Content::localized("dialogue-question-quest-courier-where").with_arg("name", npc_name),
            Marker::at(at)
                .with_label(
                    Content::localized("hud-map-character-label").with_arg("name", npc_name),
                )
                .with_kind(MarkerKind::Character)
                .with_id(target)
                .with_quest_flag(true),
            Content::localized("npc-response-quest-courier-where").with_arg("name", npc_name),
        )
    }

    /// Returns the dialogue question and response that an entity will use when
    /// the courier quest's messenger is speaking to the quest target and is
    /// attempting to finish the quest (claim the reward).
    pub fn get_courier_claim_dialogue(self) -> (Content, Content) {
        (
            Content::localized("dialogue-question-quest-courier-claim"),
            Content::localized("npc-response-quest-courier-thanks"),
        )
    }

    /// Generates a map marker that represents the position of the courier
    /// quest's targeted spot's position.
    pub fn get_quest_spot_start_marker(
        self,
        at: Vec2<f32>,
        tgt_npc_name: &str,
        quest_id: QuestId,
    ) -> Marker {
        Marker::at(at)
            .with_id(quest_id)
            .with_kind(MarkerKind::Unknown)
            .with_quest_flag(true)
            .with_label(self.get_spot_map_label(tgt_npc_name))
    }

    /// Generates a map marker that represents the position of the courier
    /// quest's target entity.
    pub fn get_quest_npc_target_marker(
        self,
        at: Vec2<f32>,
        tgt_npc_name: &str,
        target_npc_id: NpcId,
    ) -> Marker {
        Marker::at(at)
            .with_id(target_npc_id)
            .with_kind(MarkerKind::Character)
            .with_quest_flag(true)
            .with_label(
                Content::localized("hud-map-character-label").with_arg("name", tgt_npc_name),
            )
    }
}

/// Attempts to build a valid courier quest.
fn roll_courier_quest(ctx: &mut NpcCtx, messenger: Actor) -> Option<CourierQuestInstance> {
    let kind = COURIER_QUEST_VARIANTS
        .choose(&mut ctx.rng)
        .copied()
        .unwrap_or(CourierQuest::Message);

    let (target_site, target_actor) = match kind {
        // target and source are the same npc for this kind of courier quest
        CourierQuest::Deliver {
            recipient: Recipient::Giver,
            ..
        } => (ctx.npc.current_site, Actor::from(ctx.npc_id)),
        // target npc differs from source npc for these kinds of courier quests,
        // so find a target npc and the npc's site
        CourierQuest::Deliver {
            recipient: Recipient::Other,
            ..
        }
        | CourierQuest::Message => ctx
            .data
            .npcs
            .iter()
            .filter_map(|(npc_id, npc)| match &npc.role {
                Role::Civilised(Some(Profession::Hunter))
                | Role::Civilised(Some(Profession::Farmer))
                | Role::Civilised(Some(Profession::Blacksmith))
                | Role::Civilised(Some(Profession::Alchemist))
                | Role::Civilised(Some(Profession::Chef))
                | Role::Civilised(Some(Profession::Herbalist))
                | Role::Civilised(Some(Profession::Guard)) => {
                    (ctx.npc.wpos.xy().distance(npc.wpos.xy()) <= 5_000.0).then_some((npc_id, npc))
                },
                _ => None,
            })
            .choose(&mut ctx.rng)
            .and_then(|(tgt_npc_id, tgt_npc)| {
                ctx.data
                    .sites
                    .iter()
                    .filter_map(|(site_id, site)| {
                        (tgt_npc.wpos.xy().distance(site.wpos.as_()) <= 512.0).then_some(site_id)
                    })
                    .choose(&mut ctx.rng)
                    .map(|site_id| (Some(site_id), Actor::from(tgt_npc_id)))
            })?,
    };

    let spot = get_nearest_spot(ctx, kind, ctx.npc.wpos.xy().wpos_to_cpos().as_());

    // check if the payload necessitates visiting a spot. Make sure to add more
    // here later (the compiler will guide you), and avoid using `_` match arms
    // please... otherwise the compiler won't guide you
    if match kind.payload() {
        Some(Payload::GnarlingCarving) => spot.is_none(),
        Some(Payload::LegoomLeaf) | None => false,
    } {
        return None;
    }

    Some(CourierQuestInstance {
        kind,
        spot,
        source_site: ctx.npc.current_site,
        source_actor: Actor::from(ctx.npc_id),
        target_actor,
        target_site,
        messenger,
    })
}
