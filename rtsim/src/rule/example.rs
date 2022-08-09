use tracing::info;
use crate::{
    event::OnTick,
    RtState, Rule, RuleError,
};

pub struct RuleState;

impl Rule for RuleState {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        info!("Hello from example rule!");

        rtstate.bind::<Self, OnTick>(|this, rtstate, event| {
            // println!("Tick!");
        });

        Ok(Self)
    }
}
