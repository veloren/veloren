use crate::{comp, sync};
use serde_derive::{Deserialize, Serialize};
use std::marker::PhantomData;
use sum_type::sum_type;

// Automatically derive From<T> for EcsCompPacket
// for each variant EcsCompPacket::T(T.)
sum_type! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum EcsCompPacket {
        Body(comp::Body),
        Player(comp::Player),
        CanBuild(comp::CanBuild),
        Stats(comp::Stats),
        Energy(comp::Energy),
        LightEmitter(comp::LightEmitter),
        Item(comp::Item),
        Scale(comp::Scale),
        MountState(comp::MountState),
        Mounting(comp::Mounting),
        Mass(comp::Mass),
        Gravity(comp::Gravity),
        Sticky(comp::Sticky),
        AbilityAction(comp::AbilityAction),
        AbilityPool(comp::AbilityPool),
        Attacking(comp::Attacking),
        CharacterState(comp::CharacterState),
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
        Energy(PhantomData<comp::Energy>),
        LightEmitter(PhantomData<comp::LightEmitter>),
        Item(PhantomData<comp::Item>),
        Scale(PhantomData<comp::Scale>),
        MountState(PhantomData<comp::MountState>),
        Mounting(PhantomData<comp::Mounting>),
        Mass(PhantomData<comp::Mass>),
        Gravity(PhantomData<comp::Gravity>),
        Sticky(PhantomData<comp::Sticky>),
        AbilityAction(PhantomData<comp::AbilityAction>),
        AbilityPool(PhantomData<comp::AbilityPool>),
        Attacking(PhantomData<comp::Attacking>),
        CharacterState(PhantomData<comp::CharacterState>),
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
            EcsCompPacket::Energy(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::LightEmitter(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Item(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Scale(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::MountState(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Mounting(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Mass(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Gravity(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Sticky(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::AbilityAction(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::AbilityPool(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::Attacking(comp) => sync::handle_insert(comp, entity, world),
            EcsCompPacket::CharacterState(comp) => sync::handle_insert(comp, entity, world),
        }
    }

    fn apply_modify(self, entity: specs::Entity, world: &specs::World) {
        match self {
            EcsCompPacket::Body(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Player(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::CanBuild(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Stats(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Energy(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::LightEmitter(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Item(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Scale(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::MountState(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Mounting(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Mass(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Gravity(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Sticky(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::AbilityAction(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::AbilityPool(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::Attacking(comp) => sync::handle_modify(comp, entity, world),
            EcsCompPacket::CharacterState(comp) => sync::handle_modify(comp, entity, world),
        }
    }

    fn apply_remove(phantom: Self::Phantom, entity: specs::Entity, world: &specs::World) {
        match phantom {
            EcsCompPhantom::Body(_) => sync::handle_remove::<comp::Body>(entity, world),
            EcsCompPhantom::Player(_) => sync::handle_remove::<comp::Player>(entity, world),
            EcsCompPhantom::CanBuild(_) => sync::handle_remove::<comp::CanBuild>(entity, world),
            EcsCompPhantom::Stats(_) => sync::handle_remove::<comp::Stats>(entity, world),
            EcsCompPhantom::Energy(_) => sync::handle_remove::<comp::Energy>(entity, world),
            EcsCompPhantom::LightEmitter(_) => {
                sync::handle_remove::<comp::LightEmitter>(entity, world)
            },
            EcsCompPhantom::Item(_) => sync::handle_remove::<comp::Item>(entity, world),
            EcsCompPhantom::Scale(_) => sync::handle_remove::<comp::Scale>(entity, world),
            EcsCompPhantom::MountState(_) => sync::handle_remove::<comp::MountState>(entity, world),
            EcsCompPhantom::Mounting(_) => sync::handle_remove::<comp::Mounting>(entity, world),
            EcsCompPhantom::Mass(_) => sync::handle_remove::<comp::Mass>(entity, world),
            EcsCompPhantom::Gravity(_) => sync::handle_remove::<comp::Gravity>(entity, world),
            EcsCompPhantom::Sticky(_) => sync::handle_remove::<comp::Sticky>(entity, world),
            EcsCompPhantom::AbilityAction(_) => {
                sync::handle_remove::<comp::AbilityAction>(entity, world)
            },
            EcsCompPhantom::AbilityPool(_) => {
                sync::handle_remove::<comp::AbilityPool>(entity, world)
            },
            EcsCompPhantom::Attacking(_) => sync::handle_remove::<comp::Attacking>(entity, world),
            EcsCompPhantom::CharacterState(_) => {
                sync::handle_remove::<comp::CharacterState>(entity, world)
            },
        }
    }
}
