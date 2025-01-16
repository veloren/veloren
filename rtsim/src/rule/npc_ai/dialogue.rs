use super::*;

use std::sync::Arc;

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
            .then(just(move |ctx, _| {
                ctx.controller.do_idle();
                ctx.controller.dialogue_end(session);
            }))
    })
}

impl DialogueSession {
    /// Ask a question as part of a dialogue.
    ///
    /// Responses will be verified against the original response options and
    /// dialogue participant to prevent spoofing.
    pub fn ask_question<S: State, T: Into<Option<(u16, U)>>, U: Into<Response>>(
        self,
        question: Content,
        responses: impl IntoIterator<Item = T> + Clone + Send + Sync + 'static,
    ) -> impl Action<S, Option<u16>> {
        let responses = responses
            .clone()
            .into_iter()
            .flat_map(Into::into)
            .map(|(id, r)| (id, r.into()))
            .collect::<Arc<[_]>>();

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
                        // Check that the response relates the the question just asked
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
    }

    pub fn say_statement<S: State>(self, stmt: Content) -> impl Action<S> {
        talk(self.target)
            .repeat()
            // Wait for a while before making the statement to allow other dialogue to be read
            .stop_if(timeout(2.5))
            .then(now(move |ctx, _| {
                ctx.controller.dialogue_statement(self, stmt.clone());
                idle()
            }))
    }
}
