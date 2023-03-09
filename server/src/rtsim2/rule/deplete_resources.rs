use crate::rtsim2::{event::OnBlockChange, ChunkStates};
use common::{
    terrain::{CoordinateConversions, TerrainChunk},
    vol::RectRasterableVol,
};
use rtsim2::{RtState, Rule, RuleError};

pub struct DepleteResources;

impl Rule for DepleteResources {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnBlockChange>(|ctx| {
            let key = ctx.event.wpos.xy().wpos_to_cpos();
            if let Some(Some(chunk_state)) = ctx.state.resource_mut::<ChunkStates>().0.get(key) {
                let mut chunk_res = ctx.state.data().nature.get_chunk_resources(key);
                // Remove resources
                if let Some(res) = ctx.event.old.get_rtsim_resource() {
                    if chunk_state.max_res[res] > 0 {
                        chunk_res[res] = (chunk_res[res] * chunk_state.max_res[res] as f32 - 1.0)
                            .round()
                            .max(0.0)
                            / chunk_state.max_res[res] as f32;
                    }
                }
                // Add resources
                if let Some(res) = ctx.event.new.get_rtsim_resource() {
                    if chunk_state.max_res[res] > 0 {
                        chunk_res[res] = (chunk_res[res] * chunk_state.max_res[res] as f32 + 1.0)
                            .round()
                            .max(0.0)
                            / chunk_state.max_res[res] as f32;
                    }
                }
                //println!("Chunk resources = {:?}", chunk_res);
                ctx.state
                    .data_mut()
                    .nature
                    .set_chunk_resources(key, chunk_res);
            }
        });

        Ok(Self)
    }
}
