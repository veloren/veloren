pub mod character_screen;
pub mod general;
pub mod in_game;
pub mod ping;
pub mod register;
pub mod terrain;

use crate::{
    client::Client,
    sys::{loot, pets},
};
use common_ecs::{dispatch, System};
use serde::de::DeserializeOwned;
use specs::DispatcherBuilder;

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    //run ping after general, as its super fast anyway. also don't get duplicate
    // disconnect then.
    dispatch::<character_screen::Sys>(dispatch_builder, &[]);
    dispatch::<general::Sys>(dispatch_builder, &[]);
    dispatch::<in_game::Sys>(dispatch_builder, &[]);
    dispatch::<ping::Sys>(dispatch_builder, &[&general::Sys::sys_name()]);
    dispatch::<register::Sys>(dispatch_builder, &[]);
    dispatch::<terrain::Sys>(dispatch_builder, &[]);
    dispatch::<pets::Sys>(dispatch_builder, &[]);
    dispatch::<loot::Sys>(dispatch_builder, &[]);
}

/// handles all send msg and calls a handle fn
/// Aborts when a error occurred returns cnt of successful msg otherwise
pub(crate) fn try_recv_all<M, F>(
    client: &mut Client,
    stream_id: u8,
    mut f: F,
) -> Result<u64, crate::error::Error>
where
    M: DeserializeOwned,
    F: FnMut(&Client, M) -> Result<(), crate::error::Error>,
{
    let mut cnt = 0u64;
    loop {
        let msg = match client.recv(stream_id) {
            Ok(Some(msg)) => msg,
            Ok(None) => break Ok(cnt),
            Err(e) => break Err(e.into()),
        };
        if let Err(e) = f(client, msg) {
            break Err(e);
        }
        cnt += 1;
    }
}
