use super::*;

pub fn general<S: State>(tgt: Actor, session: DialogueSession) -> impl Action<S> {
    now(move |ctx, _| {
        let mut responses = Vec::new();

        // Job-dependent responses
        match &ctx.npc.job {
            // TODO: Implement hiring as a quest?
            Some(Job::Hired(by, _)) if *by == tgt => {
                responses.push((
                    Response::from(Content::localized("dialogue-cancel_hire")),
                    session
                        .say_statement(Content::localized("npc-dialogue-hire_cancelled"))
                        .then(just(move |ctx, _| ctx.controller.end_hiring()))
                        .boxed(),
                ));
            },
            Some(_) => {},
            None => {
                responses.push((
                    Response::from(Content::localized("dialogue-question-quest_req")),
                    quest::quest_request(session).boxed(),
                ));

                let can_be_hired = matches!(ctx.npc.profession(), Some(Profession::Adventurer(_)));
                if can_be_hired {
                    responses.push((
                        Response::from(Content::localized("dialogue-question-hire")),
                        dialogue::hire(tgt, session).boxed(),
                    ));
                }
            },
        }

        for quest_id in ctx.state.data().quests.related_to(ctx.npc_id) {
            let data = ctx.state.data();
            let Some(quest) = data.quests.get(quest_id) else {
                continue;
            };
            match &quest.kind {
                QuestKind::Escort {
                    escortee,
                    escorter,
                    to,
                } if *escortee == Actor::Npc(ctx.npc_id) && *escorter == tgt => {
                    let to_name = util::site_name(ctx, *to).unwrap_or_default();
                    let dst_wpos = ctx
                        .state
                        .data()
                        .sites
                        .get(*to)
                        .map_or(Vec2::zero(), |s| s.wpos.as_());
                    responses.push((
                        Response::from(Content::localized("dialogue-question-quest-escort-where")),
                        session
                            .give_marker(
                                Marker::at(dst_wpos)
                                    .with_id(quest_id)
                                    .with_label(
                                        Content::localized("hud-map-escort-label")
                                            .with_arg(
                                                "name",
                                                ctx.npc
                                                    .get_name()
                                                    .unwrap_or_else(|| "<unknown>".to_string()),
                                            )
                                            .with_arg("place", to_name.clone()),
                                    )
                                    .with_quest_flag(true),
                            )
                            .then(session.say_statement(Content::localized_with_args(
                                "npc-response-quest-escort-where",
                                [("dst", to_name)],
                            )))
                            .boxed(),
                    ));
                },
                QuestKind::Slay { target, slayer }
                    if quest.arbiter == Actor::Npc(ctx.npc_id) && *slayer == tgt =>
                {
                    // TODO: Work for non-NPCs?
                    let Actor::Npc(target_npc_id) = target else {
                        continue;
                    };
                    // Is the monster dead?
                    if let Some(target_npc) = data.npcs.get(*target_npc_id) {
                        responses.push((
                            Response::from(
                                Content::localized("dialogue-question-quest-slay-where")
                                    .with_arg("body", target_npc.body.localize_npc()),
                            ),
                            session
                                .give_marker(
                                    Marker::at(target_npc.wpos.xy())
                                        .with_id(*target)
                                        .with_label(
                                            Content::localized("hud-map-creature-label")
                                                .with_arg("body", target_npc.body.localize_npc()),
                                        )
                                        .with_quest_flag(true),
                                )
                                .then(
                                    session.say_statement(
                                        Content::localized("npc-response-quest-slay-where")
                                            .with_arg("body", target_npc.body.localize_npc()),
                                    ),
                                )
                                .boxed(),
                        ));
                    } else {
                        responses.push((
                            Response::from(Content::localized(
                                "dialogue-question-quest-slay-claim",
                            )),
                            session
                                .say_statement(Content::localized("npc-response-quest-slay-thanks"))
                                .then(now(move |ctx, _| {
                                    if let Ok(deposit) =
                                        quest::resolve_take_deposit(ctx, quest_id, true)
                                    {
                                        session
                                            .say_statement_with_gift(
                                                Content::localized("npc-response-quest-reward"),
                                                deposit,
                                            )
                                            .boxed()
                                    } else {
                                        finish().boxed()
                                    }
                                }))
                                .boxed(),
                        ));
                    }
                },
                _ => {},
            }
        }

        // General informational questions
        responses.push((
            Response::from(Content::localized("dialogue-question-site")),
            dialogue::about_site(session).boxed(),
        ));
        responses.push((
            Response::from(Content::localized("dialogue-question-self")),
            dialogue::about_self(session).boxed(),
        ));
        responses.push((
            Response::from(Content::localized("dialogue-question-sentiment")),
            dialogue::sentiments(tgt, session).boxed(),
        ));
        responses.push((
            Response::from(Content::localized("dialogue-question-directions")),
            dialogue::directions(session).boxed(),
        ));

        // Local activities
        responses.push((
            Response::from(Content::localized("dialogue-play_game")),
            dialogue::games(session).boxed(),
        ));
        // TODO: Include trading here!

        responses.push((
            Response::from(Content::localized("dialogue-finish")),
            session
                .say_statement(Content::localized("npc-goodbye"))
                .boxed(),
        ));

        session.ask_question(Content::localized("npc-question-general"), responses)
    })
}

fn about_site<S: State>(session: DialogueSession) -> impl Action<S> {
    now(move |ctx, _| {
        if let Some(site_name) = util::site_name(ctx, ctx.npc.current_site) {
            let mut action = session
                .say_statement(Content::localized_with_args("npc-info-current_site", [(
                    "site",
                    Content::Plain(site_name),
                )]))
                .boxed();

            if let Some(current_site) = ctx.npc.current_site
                && let Some(current_site) = ctx.state.data().sites.get(current_site)
            {
                for mention_site in &current_site.nearby_sites_by_size {
                    if ctx.rng.random_bool(0.5)
                        && let Some(content) = tell_site_content(ctx, *mention_site)
                    {
                        action = action.then(session.say_statement(content)).boxed();
                    }
                }
            }

            action
        } else {
            session
                .say_statement(Content::localized("npc-info-unknown"))
                .boxed()
        }
    })
}

fn about_self<S: State>(session: DialogueSession) -> impl Action<S> {
    now(move |ctx, _| {
        let name = Content::localized("npc-info-self_name")
            .with_arg("name", ctx.npc.get_name().as_deref().unwrap_or("unknown"));

        let job = ctx
            .npc
            .profession()
            .map(|p| match p {
                Profession::Farmer => "noun-role-farmer",
                Profession::Hunter => "noun-role-hunter",
                Profession::Merchant => "noun-role-merchant",
                Profession::Guard => "noun-role-guard",
                Profession::Adventurer(_) => "noun-role-adventurer",
                Profession::Blacksmith => "noun-role-blacksmith",
                Profession::Chef => "noun-role-chef",
                Profession::Alchemist => "noun-role-alchemist",
                Profession::Pirate(_) => "noun-role-pirate",
                Profession::Cultist => "noun-role-cultist",
                Profession::Herbalist => "noun-role-herbalist",
                Profession::Captain => "noun-role-captain",
            })
            .map(|p| {
                Content::localized_with_args("npc-info-role", [("role", Content::localized(p))])
            })
            .unwrap_or_else(|| Content::localized("noun-role-none"));

        let home = if let Some(site_name) = util::site_name(ctx, ctx.npc.home) {
            Content::localized_with_args("npc-info-self_home", [(
                "site",
                Content::Plain(site_name),
            )])
        } else {
            Content::localized("npc-info-self_homeless")
        };

        session
            .say_statement(name)
            .then(session.say_statement(job))
            .then(session.say_statement(home))
    })
}

fn sentiments<S: State>(tgt: Actor, session: DialogueSession) -> impl Action<S> {
    session.ask_question(Content::Plain("...".to_string()), [(
        Content::localized("dialogue-me"),
        now(move |ctx, _| {
            if ctx.sentiments.toward(tgt).is(Sentiment::ALLY) {
                session.say_statement(Content::localized("npc-response-like_you"))
            } else if ctx.sentiments.toward(tgt).is(Sentiment::RIVAL) {
                session.say_statement(Content::localized("npc-response-dislike_you"))
            } else {
                session.say_statement(Content::localized("npc-response-ambivalent_you"))
            }
        }),
    )])
}

fn hire<S: State>(tgt: Actor, session: DialogueSession) -> impl Action<S> {
    now(move |ctx, _| {
        if ctx.npc.job.is_none() && ctx.npc.rng(38792).random_bool(0.5) {
            let hire_level = match ctx.npc.profession() {
                Some(Profession::Adventurer(l)) => l,
                _ => 0,
            };
            let price_mul = 1u32 << hire_level.min(31);
            let mut responses = Vec::new();
            responses.push((
                Response::from(Content::localized("dialogue-cancel_interaction")),
                session
                    .say_statement(Content::localized("npc-response-no_problem"))
                    .boxed(),
            ));
            let options = [
                (
                    1.0,
                    60,
                    Content::localized_attr("dialogue-buy_hire_days", "day"),
                ),
                (
                    7.0,
                    300,
                    Content::localized_attr("dialogue-buy_hire_days", "week"),
                ),
            ];
            for (days, base_price, msg) in options {
                responses.push((
                    Response {
                        msg,
                        given_item: Some((
                            Arc::<ItemDef>::load_cloned("common.items.utility.coins").unwrap(),
                            price_mul.saturating_mul(base_price),
                        )),
                    },
                    session
                        .say_statement(Content::localized("npc-response-accept_hire"))
                        .then(just(move |ctx, _| {
                            ctx.controller.set_newly_hired(
                                tgt,
                                ctx.time.add_days(days, &ctx.system_data.server_constants),
                            );
                        }))
                        .boxed(),
                ));
            }
            session
                .ask_question(Content::localized("npc-response-hire_time"), responses)
                .boxed()
        } else {
            session
                .say_statement(Content::localized("npc-response-decline_hire"))
                .boxed()
        }
    })
}

fn directions<S: State>(session: DialogueSession) -> impl Action<S> {
    now(move |ctx, _| {
        let mut responses = Vec::new();

        for actor in ctx
            .state
            .data()
            .quests
            .related_actors(session.target)
            .filter(|actor| *actor != Actor::Npc(ctx.npc_id))
            .take(32)
        // Avoid mentioning too many actors
        {
            if let Some(pos) = util::locate_actor(ctx, actor)
                && let Some(name) = util::actor_name(ctx, actor)
            {
                responses.push((
                    Content::localized("dialogue-direction-actor").with_arg("name", name.clone()),
                    session
                        .give_marker(
                            Marker::at(pos.xy())
                                .with_label(
                                    Content::localized("hud-map-character-label")
                                        .with_arg("name", name.clone()),
                                )
                                .with_kind(MarkerKind::Character)
                                .with_id(actor)
                                .with_quest_flag(true),
                        )
                        .then(session.say_statement(Content::localized("npc-response-directions")))
                        .boxed(),
                ));
            }
        }

        if let Some(current_site) = ctx.npc.current_site
            && let Some(ws_id) = ctx.state.data().sites[current_site].world_site
        {
            let direction_to_nearest =
                |f: fn(&&world::site::Plot) -> bool,
                 plot_name: fn(&world::site::Plot) -> Content| {
                    now(move |ctx, _| {
                        let ws = ctx.index.sites.get(ws_id);
                        if let Some(p) = ws.plots().filter(f).min_by_key(|p| {
                            ws.tile_center_wpos(p.root_tile())
                                .distance_squared(ctx.npc.wpos.xy().as_())
                        }) {
                            session
                                .give_marker(
                                    Marker::at(ws.tile_center_wpos(p.root_tile()).as_())
                                        .with_label(plot_name(p)),
                                )
                                .then(
                                    session.say_statement(Content::localized(
                                        "npc-response-directions",
                                    )),
                                )
                                .boxed()
                        } else {
                            session
                                .say_statement(Content::localized("npc-response-doesnt_exist"))
                                .boxed()
                        }
                    })
                    .boxed()
                };

            responses.push((
                Content::localized("dialogue-direction-tavern"),
                direction_to_nearest(
                    |p| matches!(p.kind(), PlotKind::Tavern(_)),
                    |p| match p.kind() {
                        PlotKind::Tavern(t) => Content::Plain(t.name.clone()),
                        _ => unreachable!(),
                    },
                ),
            ));
            responses.push((
                Content::localized("dialogue-direction-plaza"),
                direction_to_nearest(
                    |p| matches!(p.kind(), PlotKind::Plaza(_)),
                    |_| Content::localized("hud-map-plaza"),
                ),
            ));
            responses.push((
                Content::localized("dialogue-direction-workshop"),
                direction_to_nearest(
                    |p| matches!(p.kind().meta(), Some(PlotKindMeta::Workshop { .. })),
                    |_| Content::localized("hud-map-workshop"),
                ),
            ));
            responses.push((
                Content::localized("dialogue-direction-airship_dock"),
                direction_to_nearest(
                    |p| matches!(p.kind().meta(), Some(PlotKindMeta::AirshipDock { .. })),
                    |_| Content::localized("hud-map-airship_dock"),
                ),
            ));
        }

        session.ask_question(Content::localized("npc-question-directions"), responses)
    })
}

fn rock_paper_scissors<S: State>(session: DialogueSession) -> impl Action<S> {
    now(move |ctx, _| {
        #[derive(PartialEq, Eq, Clone, Copy)]
        enum RockPaperScissor {
            Rock,
            Paper,
            Scissors,
        }
        use RockPaperScissor::*;
        impl RockPaperScissor {
            fn i18n_key(&self) -> &'static str {
                match self {
                    Rock => "dialogue-game-rock",
                    Paper => "dialogue-game-paper",
                    Scissors => "dialogue-game-scissors",
                }
            }
        }
        fn end<S: State>(
            session: DialogueSession,
            our: RockPaperScissor,
            their: RockPaperScissor,
        ) -> impl Action<S> {
            let draw = our == their;
            let we_win = matches!(
                (our, their),
                (Rock, Scissors) | (Paper, Rock) | (Scissors, Paper)
            );
            let result = if draw {
                "dialogue-game-draw"
            } else if we_win {
                "dialogue-game-win"
            } else {
                "dialogue-game-lose"
            };

            session
                .say_statement(Content::localized(our.i18n_key()))
                .then(session.say_statement(Content::localized(result)))
        }
        let choices = [Rock, Paper, Scissors];
        let our_choice = choices
            .choose(&mut ctx.rng)
            .expect("We have a non-empty array");

        let choices = choices.map(|choice| {
            (
                Response::from(Content::localized(choice.i18n_key())),
                end(session, *our_choice, choice),
            )
        });

        session.ask_question(
            Content::localized("dialogue-game-rock_paper_scissors"),
            choices,
        )
    })
}

fn games<S: State>(session: DialogueSession) -> impl Action<S> {
    let games = [(
        Response::from(Content::localized("dialogue-game-rock_paper_scissors")),
        rock_paper_scissors(session),
    )];

    session.ask_question(Content::localized("dialogue-game-what_game"), games)
}
