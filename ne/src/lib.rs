#![allow(dead_code)]

mod error;
mod message;
mod mailsender;

pub use error::{NetworkError, NetworkResult};
pub use mailsender::MailSender;
