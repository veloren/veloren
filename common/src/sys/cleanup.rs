use crate::comp::Controller;
use specs::{Entities, Join, System, WriteStorage};
use vek::*;

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (Entities<'a>, WriteStorage<'a, Controller>);

    fn run(&mut self, (entities, mut controllers): Self::SystemData) {
        for controller in (&mut controllers).join() {
            *controller = Controller::default();
        }
    }
}
