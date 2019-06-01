use crate::{comp, state};
use serde_derive::{Deserialize, Serialize};
use std::marker::PhantomData;

// Automatically derive From<T> for EcsResPacket
// for each variant EcsResPacket::T(T).
sphynx::sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsResPacket {
        Time(state::Time),
        TimeOfDay(state::TimeOfDay),
    }
}
impl sphynx::ResPacket for EcsResPacket {}
// Automatically derive From<T> for EcsCompPacket
// for each variant EcsCompPacket::T(T.)
sphynx::sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsCompPacket {
        Pos(comp::phys::Pos),
        Vel(comp::phys::Vel),
        Ori(comp::phys::Ori),
        Actor(comp::Actor),
        Player(comp::Player),
        Stats(comp::Stats),
        Attacking(comp::Attacking),
    }
}
// Automatically derive From<T> for EcsCompPhantom
// for each variant EcsCompPhantom::T(PhantomData<T>).
sphynx::sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsCompPhantom {
        Pos(PhantomData<comp::phys::Pos>),
        Vel(PhantomData<comp::phys::Vel>),
        Ori(PhantomData<comp::phys::Ori>),
        Actor(PhantomData<comp::Actor>),
        Player(PhantomData<comp::Player>),
        Stats(PhantomData<comp::Stats>),
        Attacking(PhantomData<comp::Attacking>),
    }
}
impl sphynx::CompPacket for EcsCompPacket {
    type Phantom = EcsCompPhantom;
}
