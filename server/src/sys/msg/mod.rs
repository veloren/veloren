pub mod character_screen;
pub mod general;
pub mod in_game;
pub mod ping;
pub mod register;

use crate::streams::GetStream;

/// handles all send msg and calls a handle fn
/// Aborts when a error occurred returns cnt of successful msg otherwise
pub(in crate::sys::msg) fn try_recv_all<T, F>(
    stream: &mut T,
    mut f: F,
) -> Result<u64, crate::error::Error>
where
    T: GetStream,
    F: FnMut(&mut T, T::RecvMsg) -> Result<(), crate::error::Error>,
{
    let mut cnt = 0u64;
    loop {
        let msg = match stream.get_mut().try_recv() {
            Ok(Some(msg)) => msg,
            Ok(None) => break Ok(cnt),
            Err(e) => break Err(e.into()),
        };
        if let Err(e) = f(stream, msg) {
            break Err(e);
        }
        cnt += 1;
    }
}
