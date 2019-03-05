pub mod data;
pub mod error;
pub mod post;
pub mod postbox;
pub mod postoffice;
mod test;

// Reexports
pub use self::{
    data::{ClientMsg, ServerMsg},
    post::{
        Error as PostError,
        PostBox,
        PostOffice,
    },
};

pub trait PostSend = 'static + serde::Serialize + std::marker::Send + std::fmt::Debug;
pub trait PostRecv = 'static + serde::de::DeserializeOwned + std::marker::Send + std::fmt::Debug;
