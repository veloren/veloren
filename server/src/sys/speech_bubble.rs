use super::SysTimer;
use common::{comp::SpeechBubble, state::Time};
use specs::{Entities, Join, Read, System, Write, WriteStorage};

/// This system removes timed-out speech bubbles
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, Time>,
        WriteStorage<'a, SpeechBubble>,
        Write<'a, SysTimer<Self>>,
    );

    fn run(&mut self, (entities, time, mut speech_bubbles, mut timer): Self::SystemData) {
        timer.start();

        let expired_ents: Vec<_> = (&entities, &mut speech_bubbles)
            .join()
            .filter(|(_, speech_bubble)| speech_bubble.timeout.map_or(true, |t| t.0 < time.0))
            .map(|(ent, _)| ent)
            .collect();
        for ent in expired_ents {
            println!("Remoaving bobble");
            speech_bubbles.remove(ent);
        }

        timer.end();
    }
}
