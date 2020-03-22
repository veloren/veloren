/*
use ::uvth::ThreadPool;
use bincode;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

pub struct SerializeFuture {
    shared_state: Arc<Mutex<SerializeSharedState>>,
}

struct SerializeSharedState {
    result: Option<Vec<u8>>,
    waker: Option<Waker>,
}

pub struct DeserializeFuture<M: 'static + Send + DeserializeOwned> {
    shared_state: Arc<Mutex<DeserializeSharedState<M>>>,
}

struct DeserializeSharedState<M: 'static + Send + DeserializeOwned> {
    result: Option<M>,
    waker: Option<Waker>,
}

impl Future for SerializeFuture {
    type Output = Vec<u8>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.result.is_some() {
            Poll::Ready(shared_state.result.take().unwrap())
        } else {
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl SerializeFuture {
    pub fn new<M: 'static + Send + Serialize>(message: M, pool: &ThreadPool) -> Self {
        let shared_state = Arc::new(Mutex::new(SerializeSharedState {
            result: None,
            waker: None,
        }));
        // Spawn the new thread
        let thread_shared_state = shared_state.clone();
        pool.execute(move || {
            let mut writer = {
                let actual_size = bincode::serialized_size(&message).unwrap();
                Vec::<u8>::with_capacity(actual_size as usize)
            };
            if let Err(e) = bincode::serialize_into(&mut writer, &message) {
                panic!(
                    "bincode serialize error, probably undefined behavior somewhere else, check \
                     the possible error types of `bincode::serialize_into`: {}",
                    e
                );
            };

            let mut shared_state = thread_shared_state.lock().unwrap();
            shared_state.result = Some(writer);
            if let Some(waker) = shared_state.waker.take() {
                waker.wake()
            }
        });

        Self { shared_state }
    }
}

impl<M: 'static + Send + DeserializeOwned> Future for DeserializeFuture<M> {
    type Output = M;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.result.is_some() {
            Poll::Ready(shared_state.result.take().unwrap())
        } else {
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<M: 'static + Send + DeserializeOwned> DeserializeFuture<M> {
    pub fn new(data: Vec<u8>, pool: &ThreadPool) -> Self {
        let shared_state = Arc::new(Mutex::new(DeserializeSharedState {
            result: None,
            waker: None,
        }));
        // Spawn the new thread
        let thread_shared_state = shared_state.clone();
        pool.execute(move || {
            let decoded: M = bincode::deserialize(data.as_slice()).unwrap();

            let mut shared_state = thread_shared_state.lock().unwrap();
            shared_state.result = Some(decoded);
            if let Some(waker) = shared_state.waker.take() {
                waker.wake()
            }
        });

        Self { shared_state }
    }
}
*/
/*
#[cfg(test)]
mod tests {
    use crate::{
        async_serde::*,
        message::{MessageBuffer, OutGoingMessage},
        types::{Frame, Sid},
    };
    use std::{collections::VecDeque, sync::Arc};
    use uvth::ThreadPoolBuilder;

    use async_std::{
        io::BufReader,
        net::{TcpListener, TcpStream, ToSocketAddrs},
        prelude::*,
        task,
    };
    #[macro_use] use futures;

    async fn tick_tock(msg: String, pool: &ThreadPool) {
        let serialized = SerializeFuture::new(msg.clone(), pool).await;
        let deserialized = DeserializeFuture::<String>::new(serialized, pool).await;
        assert_eq!(msg, deserialized)
    }

    #[test]
    fn multiple_serialize() {
        let msg = "ThisMessageisexactly100charactersLongToPrecislyMeassureSerialisation_SoYoucanSimplyCountThe123inhere".to_string();
        let pool = ThreadPoolBuilder::new().build();
        let (r1, r2, r3) = task::block_on(async {
            let s1 = SerializeFuture::new(msg.clone(), &pool);
            let s2 = SerializeFuture::new(msg.clone(), &pool);
            let s3 = SerializeFuture::new(msg.clone(), &pool);
            futures::join!(s1, s2, s3)
        });
        assert_eq!(r1.len(), 108);
        assert_eq!(r2.len(), 108);
        assert_eq!(r3.len(), 108);
    }

    #[test]
    fn await_serialize() {
        let msg = "ThisMessageisexactly100charactersLongToPrecislyMeassureSerialisation_SoYoucanSimplyCountThe123inhere".to_string();
        let pool = ThreadPoolBuilder::new().build();
        task::block_on(async {
            let r1 = SerializeFuture::new(msg.clone(), &pool).await;
            let r2 = SerializeFuture::new(msg.clone(), &pool).await;
            let r3 = SerializeFuture::new(msg.clone(), &pool).await;
            assert_eq!(r1.len(), 108);
            assert_eq!(r2.len(), 108);
            assert_eq!(r3.len(), 108);
        });
    }

    #[test]
    fn multiple_serialize_deserialize() {
        let msg = "ThisMessageisexactly100charactersLongToPrecislyMeassureSerialisation_SoYoucanSimplyCountThe123inhere".to_string();
        let pool = ThreadPoolBuilder::new().build();
        task::block_on(async {
            let s1 = tick_tock(msg.clone(), &pool);
            let s2 = tick_tock(msg.clone(), &pool);
            let s3 = tick_tock(msg.clone(), &pool);
            futures::join!(s1, s2, s3)
        });
    }
}
*/
