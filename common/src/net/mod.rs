pub mod data;
//pub mod post;
pub mod post2;

pub use post2 as post;

// Reexports
pub use self::{
    data::{ClientMsg, ServerMsg},
    post::{Error as PostError, PostBox, PostOffice},
};
