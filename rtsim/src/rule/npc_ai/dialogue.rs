use super::*;

pub fn talk<S: State>(tgt: Actor) -> impl Action<S> + Clone {
    just(move |ctx, _| ctx.controller.do_talk(tgt)).debug(|| "talking")
}

pub fn do_dialogue<S: State, A: Action<S>>(
    tgt: Actor,
    f: impl Fn(DialogueSession) -> A + Send + Sync + 'static,
) -> impl Action<S> {
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
            // If all else fails, add a timeout to dialogues
            // TODO: Only timeout if no messages have been received recently
            .stop_if(timeout(60.0))
            .then(just(move |ctx, _| {
                ctx.controller.do_idle();
                ctx.controller.dialogue_end(session);
            }))
    })
}

impl DialogueSession {
    pub fn ask_question<S: State, O: Into<Option<(u16, Content)>>>(
        self,
        question: Content,
        options: impl IntoIterator<Item = O> + Clone + Send + Sync + 'static,
    ) -> impl Action<S, u16> {
        now(move |ctx, _| {
            let q_tag = ctx.controller.dialogue_question(
                self,
                question.clone(),
                options.clone().into_iter().flat_map(Into::into),
            );
            until(move |ctx, _| {
                let mut response = None;
                ctx.inbox.retain(|input| {
                    if let NpcInput::Dialogue(_, dialogue) = input
                        && dialogue.id == self.id
                        && let DialogueKind::Response { tag, option_id, .. } = dialogue.kind
                        && tag == q_tag
                    {
                        response = Some(option_id);
                        false
                    } else {
                        true
                    }
                });
                match response {
                    // TODO: Should be 'engage target in conversation'
                    None => ControlFlow::Continue(talk(self.target)),
                    Some(option_id) => ControlFlow::Break(option_id),
                }
            })
        })
            // Add some thinking time after hearing a response
            .and_then(move |option_id| talk(self.target).repeat().stop_if(timeout(0.5)).map(move |_, _| option_id))
    }

    pub fn say_statement<S: State>(self, stmt: Content) -> impl Action<S> {
        now(move |ctx, _| {
            ctx.controller.dialogue_statement(self, stmt.clone());
            // Wait for a while after making the statement so others can read it
            talk(self.target)
                .repeat()
                .stop_if(timeout(2.5))
                .map(|_, _| ())
        })
    }
}
