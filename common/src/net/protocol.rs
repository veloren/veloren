// Parent
use super::packet::{Frame};
use super::Error;

pub trait Protocol {
    fn send(&self, frame: Frame) -> Result<(), Error>;
    fn recv(&self) -> Result<Frame, Error>;
}
