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
        Pos(comp::Pos),
        Vel(comp::Vel),
        Ori(comp::Ori),
        Body(comp::Body),
        Player(comp::Player),
        CanBuild(comp::CanBuild),
        Stats(comp::Stats),
        LightEmitter(comp::LightEmitter),
        Item(comp::Item),
        Scale(comp::Scale),
        MountState(comp::MountState),
        Mounting(comp::Mounting),
        Mass(comp::Mass),
        Projectile(comp::Projectile),
        Gravity(comp::Gravity),
        Sticky(comp::Sticky),
    }
}
// Automatically derive From<T> for EcsCompPhantom
// for each variant EcsCompPhantom::T(PhantomData<T>).
sphynx::sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsCompPhantom {
        Pos(PhantomData<comp::Pos>),
        Vel(PhantomData<comp::Vel>),
        Ori(PhantomData<comp::Ori>),
        Body(PhantomData<comp::Body>),
        Player(PhantomData<comp::Player>),
        CanBuild(PhantomData<comp::CanBuild>),
        Stats(PhantomData<comp::Stats>),
        LightEmitter(PhantomData<comp::LightEmitter>),
        Item(PhantomData<comp::Item>),
        Scale(PhantomData<comp::Scale>),
        MountState(PhantomData<comp::MountState>),
        Mounting(PhantomData<comp::Mounting>),
        Mass(PhantomData<comp::Mass>),
        Projectile(PhantomData<comp::Projectile>),
        Gravity(PhantomData<comp::Gravity>),
        Sticky(PhantomData<comp::Sticky>),
    }
}
impl sphynx::CompPacket for EcsCompPacket {
    type Phantom = EcsCompPhantom;
}
