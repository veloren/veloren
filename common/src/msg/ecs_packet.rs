use std::marker::PhantomData;
use serde_derive::{Serialize, Deserialize};
use crate::comp;

// Automatically derive From<T> for Packet for each variant Packet::T(T)
sphynx::sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsPacket {
        Pos(comp::phys::Pos),
        Vel(comp::phys::Vel),
        Dir(comp::phys::Dir),
        Character(comp::Character),
        Player(comp::Player),
    }
}
// Automatically derive From<T> for Phantom for each variant Phantom::T(PhantomData<T>)
sphynx::sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsPhantom {
        Pos(PhantomData<comp::phys::Pos>),
        Vel(PhantomData<comp::phys::Vel>),
        Dir(PhantomData<comp::phys::Dir>),
        Character(PhantomData<comp::Character>),
        Player(PhantomData<comp::Player>),
    }
}
impl sphynx::Packet for EcsPacket {
    type Phantom = EcsPhantom;
}
