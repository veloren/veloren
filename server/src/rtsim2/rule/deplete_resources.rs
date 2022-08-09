use tracing::info;
use rtsim2::{RtState, Rule, RuleError};
use crate::rtsim2::{
    event::OnBlockChange,
    ChunkStates,
};
use common::{
    terrain::TerrainChunk,
    vol::RectRasterableVol,
};

pub struct State;

impl Rule for State {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        info!("Hello from the resource depletion rule!");

        rtstate.bind::<Self, OnBlockChange>(|this, rtstate, event| {
            let key = event.wpos
                .xy()
                .map2(TerrainChunk::RECT_SIZE, |e, sz| e.div_euclid(sz as i32));
            if let Some(Some(chunk_state)) = rtstate.resource_mut::<ChunkStates>().0.get(key) {
                let mut chunk_res = rtstate.data().nature.get_chunk_resources(key);
                // Remove resources
                if let Some(res) = event.old.get_rtsim_resource() {
                    if chunk_state.max_res[res] > 0 {
                        chunk_res[res] = (chunk_res[res] * chunk_state.max_res[res] as f32 - 1.0)
                            .round()
                            .max(0.0) / chunk_state.max_res[res] as f32;
                    }
                }
                // Add resources
                if let Some(res) = event.new.get_rtsim_resource() {
                    if chunk_state.max_res[res] > 0 {
                        chunk_res[res] = (chunk_res[res] * chunk_state.max_res[res] as f32 + 1.0)
                            .round()
                            .max(0.0) / chunk_state.max_res[res] as f32;
                    }
                }
                println!("Chunk resources = {:?}", chunk_res);
                rtstate.data_mut().nature.set_chunk_resources(key, chunk_res);
            }
        });

        Ok(Self)
    }
}
