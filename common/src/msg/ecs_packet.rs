use crate::{comp, state};
use serde_derive::{Deserialize, Serialize};
use sphynx::sum_type;
use std::marker::PhantomData;

// Automatically derive From<T> for EcsResPacket
// for each variant EcsResPacket::T(T).
sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsResPacket {
        Time(state::Time),
        TimeOfDay(state::TimeOfDay),
    }
}
impl sphynx::ResPacket for EcsResPacket {
    fn apply(self, world: &specs::World) {
        match self {
            EcsResPacket::Time(time) => sphynx::handle_res_update(time, world),
            EcsResPacket::TimeOfDay(time_of_day) => sphynx::handle_res_update(time_of_day, world),
        }
    }
}
// Automatically derive From<T> for EcsCompPacket
// for each variant EcsCompPacket::T(T.)
sum_type! {
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
sum_type! {
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
    fn apply_insert(self, entity: specs::Entity, world: &specs::World) {
        match self {
            EcsCompPacket::Pos(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Vel(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Ori(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Body(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Player(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::CanBuild(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Stats(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::LightEmitter(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Item(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Scale(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::MountState(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Mounting(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Mass(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Projectile(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Gravity(comp) => sphynx::handle_insert(comp, entity, world),
            EcsCompPacket::Sticky(comp) => sphynx::handle_insert(comp, entity, world),
        }
    }
    fn apply_modify(self, entity: specs::Entity, world: &specs::World) {
        match self {
            EcsCompPacket::Pos(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Vel(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Ori(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Body(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Player(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::CanBuild(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Stats(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::LightEmitter(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Item(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Scale(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::MountState(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Mounting(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Mass(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Projectile(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Gravity(comp) => sphynx::handle_modify(comp, entity, world),
            EcsCompPacket::Sticky(comp) => sphynx::handle_modify(comp, entity, world),
        }
    }
    fn apply_remove(phantom: Self::Phantom, entity: specs::Entity, world: &specs::World) {
        match phantom {
            EcsCompPhantom::Pos(_) => sphynx::handle_remove::<comp::Pos>(entity, world),
            EcsCompPhantom::Vel(_) => sphynx::handle_remove::<comp::Vel>(entity, world),
            EcsCompPhantom::Ori(_) => sphynx::handle_remove::<comp::Ori>(entity, world),
            EcsCompPhantom::Body(_) => sphynx::handle_remove::<comp::Body>(entity, world),
            EcsCompPhantom::Player(_) => sphynx::handle_remove::<comp::Player>(entity, world),
            EcsCompPhantom::CanBuild(_) => sphynx::handle_remove::<comp::CanBuild>(entity, world),
            EcsCompPhantom::Stats(_) => sphynx::handle_remove::<comp::Stats>(entity, world),
            EcsCompPhantom::LightEmitter(_) => {
                sphynx::handle_remove::<comp::LightEmitter>(entity, world)
            }
            EcsCompPhantom::Item(_) => sphynx::handle_remove::<comp::Item>(entity, world),
            EcsCompPhantom::Scale(_) => sphynx::handle_remove::<comp::Scale>(entity, world),
            EcsCompPhantom::MountState(_) => {
                sphynx::handle_remove::<comp::MountState>(entity, world)
            }
            EcsCompPhantom::Mounting(_) => sphynx::handle_remove::<comp::Mounting>(entity, world),
            EcsCompPhantom::Mass(_) => sphynx::handle_remove::<comp::Mass>(entity, world),
            EcsCompPhantom::Projectile(_) => {
                sphynx::handle_remove::<comp::Projectile>(entity, world)
            }
            EcsCompPhantom::Gravity(_) => sphynx::handle_remove::<comp::Gravity>(entity, world),
            EcsCompPhantom::Sticky(_) => sphynx::handle_remove::<comp::Sticky>(entity, world),
        }
    }
}
