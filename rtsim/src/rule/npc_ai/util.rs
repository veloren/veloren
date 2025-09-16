use super::*;

pub fn site_name(ctx: &NpcCtx, site_id: impl Into<Option<SiteId>>) -> Option<String> {
    let world_site = ctx.data.sites.get(site_id.into()?)?.world_site?;
    Some(ctx.index.sites.get(world_site).name()?.to_string())
}

pub fn locate_actor(ctx: &NpcCtx, actor: Actor) -> Option<Vec3<f32>> {
    match actor {
        Actor::Npc(npc_id) => ctx.data.npcs.get(npc_id).map(|npc| npc.wpos),
        Actor::Character(character_id) => ctx
            .system_data
            .id_maps
            .character_entity(character_id)
            .and_then(|c| ctx.system_data.positions.get(c))
            .map(|p| p.0),
    }
}

pub fn actor_exists(ctx: &NpcCtx, actor: Actor) -> bool {
    match actor {
        Actor::Npc(npc_id) => ctx.data.npcs.contains_key(npc_id),
        Actor::Character(character_id) => ctx
            .system_data
            .id_maps
            .character_entity(character_id)
            .is_some(),
    }
}

pub fn actor_name(ctx: &NpcCtx, actor: Actor) -> Option<String> {
    match actor {
        Actor::Npc(npc_id) => ctx.data.npcs.get(npc_id).and_then(|npc| npc.get_name()),
        // TODO
        Actor::Character(_) => None,
    }
}

pub fn talk<S: State>(tgt: Actor) -> impl Action<S> + Clone {
    just(move |ctx, _| ctx.controller.do_talk(tgt)).debug(|| "talking")
}

pub fn do_dialogue<S: State, T: Default + Clone + Send + Sync + 'static, A: Action<S, T>>(
    tgt: Actor,
    f: impl Fn(DialogueSession) -> A + Clone + Send + Sync + 'static,
) -> impl Action<S, T> {
    now(move |ctx, _| {
        let session = ctx.controller.dialogue_start(tgt);
        f(session)
            // If an end dialogue message is received, stop the dialogue
            .stop_if(move |ctx: &mut NpcCtx| {
                let mut stop = false;
                ctx.inbox.retain(|input| {
                    if let NpcInput::Dialogue(_, dialogue) = input
                        && dialogue.id == session.id
                        && let DialogueKind::End = dialogue.kind
                    {
                        stop = true;
                        false
                    } else {
                        true
                    }
                });
                stop
            })
            // As a backstop, timeout after 10 minutes of dialogue to avoid accidentally soft-locking NPCs
            .stop_if(timeout(600.0))
            .and_then(move |x: Option<Option<T>>| just(move |ctx, _| {
                ctx.controller.do_idle();
                ctx.controller.dialogue_end(session);
                x.clone().flatten().unwrap_or_default()
            }))
            // Handle early cancellation elegantly
            .when_cancelled(move |ctx: &mut NpcCtx| ctx.controller.dialogue_end(session))
    })
    .with_important_priority()
}

impl DialogueSession {
    /// Ask a question as part of a dialogue.
    ///
    /// Responses will be verified against the original response options and
    /// dialogue participant to prevent spoofing.
    pub fn ask_question<
        S: State,
        R: Into<Response>,
        T: Default + Send + Sync + 'static,
        A: Action<S, T>,
    >(
        self,
        question: Content,
        responses: impl IntoIterator<Item = (R, A)> + Send + Sync + 'static,
    ) -> impl Action<S, T> {
        let (responses, actions): (Vec<_>, Vec<_>) = responses
            .into_iter()
            .enumerate()
            .map(|(idx, (r, a))| ((idx as u16, r.into()), a))
            .unzip();

        let actions_once = Arc::new(take_once::TakeOnce::new());
        let _ = actions_once.store(actions);

        now(move |ctx, _| {
            let q_tag = ctx.controller.dialogue_question(
                self,
                question.clone(),
                responses.iter().cloned(),
            );
            let responses = responses.clone();
            until(move |ctx, _| {
                let mut id = None;
                ctx.inbox.retain(|input| {
                    if let NpcInput::Dialogue(_, dialogue) = input
                        // Check that the response is for the same dialogue
                        && dialogue.id == self.id
                        && let DialogueKind::Response { tag, response_id, response, .. } = &dialogue.kind
                        // Check that the response relates to the question just asked
                        && *tag == q_tag
                        // Check that the response matches one of our requested responses
                        && responses.iter().any(|(r_id, r)| r_id == response_id && r == response)
                    {
                        id = Some(*response_id);
                        false
                    } else {
                        true
                    }
                });
                match id {
                    // TODO: Should be 'engage target in conversation'
                    None => ControlFlow::Continue(talk(self.target)),
                    Some(response_id) => ControlFlow::Break(response_id),
                }
            })
        })
            // Add some thinking time after hearing a response
            .and_then(move |response_id| talk(self.target).repeat().stop_if(timeout(0.5)).map(move |_, _| response_id))
            // If all else fails, add a timeout to dialogues
            // TODO: Only timeout if no messages have been received recently
            .stop_if(timeout(60.0))
            .and_then(move |resp: Option<u16>| {
                if let Some(action) = resp.and_then(|resp| actions_once.take().unwrap().into_iter().nth(resp as usize)) {
                    action.map(|x, _| x).boxed()
                } else {
                    idle().map(|_, _| Default::default()).boxed()
                }
            })
    }

    pub fn ask_yes_no_question<S: State>(self, question: Content) -> impl Action<S, bool> {
        let answer = |is_yes| just(move |_, _| is_yes);
        self.ask_question(question, [
            (Content::localized("common-yes"), answer(true)),
            (Content::localized("common-no"), answer(false)),
        ])
    }

    pub fn say_statement_with_gift<S: State>(
        self,
        stmt: Content,
        item: Option<(Arc<ItemDef>, u32)>,
    ) -> impl Action<S> {
        now(move |ctx, _| {
            let s_tag = ctx
                .controller
                .dialogue_statement(self, stmt.clone(), item.clone());
            // Wait for the ack
            talk(self.target)
            .repeat()
            // Wait for a while before accepting the ACK to avoid dialogue progressing too fast
            .stop_if(timeout(1.5))
            // Wait for the ACK
            .then(until(move |ctx, _| {
                    let mut acked = false;
                    ctx.inbox.retain(|input| {
                        if let NpcInput::Dialogue(_, dialogue) = input
                            // Check that the response is for the same dialogue
                            && dialogue.id == self.id
                            && let DialogueKind::Ack { tag } = &dialogue.kind
                            // Check that the ACL relates to the statement just given
                            && *tag == s_tag
                        {
                            acked = true;
                            false
                        } else {
                            true
                        }
                    });
                    if acked {
                       ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(talk(self.target))
                    }
                })
                    // As a final safeguard, timeout after a while
                    .stop_if(timeout(30.0)))
        })
        .map(|_, _| ())
    }

    pub fn say_statement<S: State>(self, stmt: Content) -> impl Action<S> {
        self.say_statement_with_gift(stmt, None)
    }

    pub fn give_marker<S: State>(self, marker: Marker) -> impl Action<S> {
        just(move |ctx, _| ctx.controller.dialogue_marker(self, marker.clone()))
    }
}
