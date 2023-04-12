use crate::rtsim::{event::OnBlockChange, ChunkStates};
use common::terrain::CoordinateConversions;
use rtsim::{RtState, Rule, RuleError};

pub struct DepleteResources;

impl Rule for DepleteResources {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnBlockChange>(|ctx| {
            let chunk_states = ctx.state.resource::<ChunkStates>();
            let mut data = ctx.state.data_mut();
            for change in &ctx.event.changes {
                let key = change.wpos.xy().wpos_to_cpos();
                if let Some(Some(chunk_state)) = chunk_states.0.get(key) {
                    let mut chunk_res = data.nature.get_chunk_resources(key);
                    // Remove resources
                    if let Some(res) = change.old.get_rtsim_resource() {
                        if chunk_state.max_res[res] > 0 {
                            chunk_res[res] = (chunk_res[res] * chunk_state.max_res[res] as f32
                                - 1.0)
                                .round()
                                .max(0.0)
                                / chunk_state.max_res[res] as f32;
                        }
                    }
                    // Replenish resources
                    if let Some(res) = change.new.get_rtsim_resource() {
                        if chunk_state.max_res[res] > 0 {
                            chunk_res[res] = (chunk_res[res] * chunk_state.max_res[res] as f32
                                + 1.0)
                                .round()
                                .max(0.0)
                                / chunk_state.max_res[res] as f32;
                        }
                    }

                    data.nature.set_chunk_resources(key, chunk_res);
                }
            }
        });

        Ok(Self)
    }
}
