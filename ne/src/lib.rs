#![allow(dead_code)]

mod error;
mod message;
mod mailsender;
mod mailbox;

pub use error::{NetworkError, NetworkResult};
pub use mailsender::MailSender;
pub use message::Reliability;
pub use mailbox::{Mail, MailBox};
