use crate::comp;
use serde_derive::{Deserialize, Serialize};
use std::marker::PhantomData;

// Automatically derive From<T> for Packet for each variant Packet::T(T)
sphynx::sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsPacket {
        Pos(comp::phys::Pos),
        Vel(comp::phys::Vel),
        Dir(comp::phys::Dir),
        Actor(comp::Actor),
        Player(comp::Player),
        Stats(comp::Stats),
    }
}
// Automatically derive From<T> for Phantom for each variant Phantom::T(PhantomData<T>)
sphynx::sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsPhantom {
        Pos(PhantomData<comp::phys::Pos>),
        Vel(PhantomData<comp::phys::Vel>),
        Dir(PhantomData<comp::phys::Dir>),
        Actor(PhantomData<comp::Actor>),
        Player(PhantomData<comp::Player>),
        Stats(PhantomData<comp::Stats>),
    }
}
impl sphynx::Packet for EcsPacket {
    type Phantom = EcsPhantom;
}
