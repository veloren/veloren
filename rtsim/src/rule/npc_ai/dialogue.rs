use super::*;

pub fn do_dialogue<S: State, A: Action<S>>(
    tgt: Actor,
    f: impl Fn(DialogueSession) -> A + Send + Sync + 'static,
) -> impl Action<S> {
    now(move |ctx, _| {
        let session = ctx.controller.dialogue_start(tgt);
        f(session)
            // TODO: Stop conversation if player walks away
            // .stop_if(||)
            .then(just(move |ctx, _| {
                ctx.controller.dialogue_end(session);
            }))
    })
}

impl DialogueSession {
    pub fn ask_question<S: State>(
        self,
        question: Content,
        options: impl IntoIterator<Item = (u16, Content)> + Clone + Send + Sync + 'static,
    ) -> impl Action<S, u16> {
        now(move |ctx, _| {
            let q_tag = ctx
                .controller
                .dialogue_question(self, question.clone(), options.clone());
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
                    None => ControlFlow::Continue(idle().boxed()),
                    Some(option_id) => ControlFlow::Break(option_id),
                }
            })
        })
    }

    pub fn say_statement<S: State>(self, stmt: Content) -> impl Action<S> {
        now(move |ctx, _| {
            ctx.controller.dialogue_statement(self, stmt.clone());
            // Wait for a while after making the statement so others can read it
            idle().repeat().stop_if(timeout(4.0)).map(|_, _| ())
        })
    }
}
