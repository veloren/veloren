use crate::{comp, state, sync};
use serde_derive::{Deserialize, Serialize};
use std::marker::PhantomData;
use sum_type::sum_type;

// Automatically derive From<T> for EcsResPacket
// for each variant EcsResPacket::T(T).
sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsResPacket {
        Time(state::Time),
        TimeOfDay(state::TimeOfDay),
    }
}
impl sync::ResPacket for EcsResPacket {
    fn apply(self, world: &specs::World) {
        match self {
            EcsResPacket::Time(time) => sync::handle_res_update(time, world),
            EcsResPacket::TimeOfDay(time_of_day) => sync::handle_res_update(time_of_day, world),
        }
    }
}
// Automatically derive From<T> for EcsCompPacket
// for each variant EcsCompPacket::T(T.)
sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsCompPacket {
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
sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsCompPhantom {
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
impl sync::CompPacket for EcsCompPacket {
    type Phantom = EcsCompPhantom;
    fn apply_insert(self, entity: specs::Entity, world: &specs::World) {
        match self {
            EcsCompPacket::Body(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Player(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::CanBuild(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Stats(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::LightEmitter(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Item(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Scale(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::MountState(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Mounting(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Mass(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Projectile(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Gravity(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Sticky(comp) => sync::handle_insert(comp, entity, world),
        }
    }
    fn apply_modify(self, entity: specs::Entity, world: &specs::World) {
        match self {
            EcsCompPacket::Body(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Player(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::CanBuild(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Stats(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::LightEmitter(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Item(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Scale(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::MountState(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Mounting(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Mass(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Projectile(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Gravity(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Sticky(comp) => sync::handle_modify(comp, entity, world),
        }
    }
    fn apply_remove(phantom: Self::Phantom, entity: specs::Entity, world: &specs::World) {
        match phantom {
            EcsCompPhantom::Body(_) => sync::handle_remove::<comp::Body>(entity, world),
            EcsCompPhantom::Player(_) => sync::handle_remove::<comp::Player>(entity, world),
            EcsCompPhantom::CanBuild(_) => sync::handle_remove::<comp::CanBuild>(entity, world),
            EcsCompPhantom::Stats(_) => sync::handle_remove::<comp::Stats>(entity, world),
            EcsCompPhantom::LightEmitter(_) => {
                sync::handle_remove::<comp::LightEmitter>(entity, world)
            }
            EcsCompPhantom::Item(_) => sync::handle_remove::<comp::Item>(entity, world),
            EcsCompPhantom::Scale(_) => sync::handle_remove::<comp::Scale>(entity, world),
            EcsCompPhantom::MountState(_) => sync::handle_remove::<comp::MountState>(entity, world),
            EcsCompPhantom::Mounting(_) => sync::handle_remove::<comp::Mounting>(entity, world),
            EcsCompPhantom::Mass(_) => sync::handle_remove::<comp::Mass>(entity, world),
            EcsCompPhantom::Projectile(_) => sync::handle_remove::<comp::Projectile>(entity, world),
            EcsCompPhantom::Gravity(_) => sync::handle_remove::<comp::Gravity>(entity, world),
            EcsCompPhantom::Sticky(_) => sync::handle_remove::<comp::Sticky>(entity, world),
        }
    }
}
