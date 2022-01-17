use crate::sync::{self, NetSync};
use common::comp;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

macro_rules! comp_packet {
    ($($component_name:ident: $component_type:ident,)*) => {
        sum_type::sum_type! {
            #[derive(Clone, Debug, Serialize, Deserialize)]
            pub enum EcsCompPacket {
                $($component_type($component_type),)*
                Pos(comp::Pos),
                Vel(comp::Vel),
                Ori(comp::Ori),
            }
        }

        sum_type::sum_type! {
            #[derive(Clone, Debug, Serialize, Deserialize)]
            pub enum EcsCompPhantom {
                $($component_type(PhantomData<$component_type>),)*
                Pos(PhantomData<comp::Pos>),
                Vel(PhantomData<comp::Vel>),
                Ori(PhantomData<comp::Ori>),
            }
        }

        impl sync::CompPacket for EcsCompPacket {
            type Phantom = EcsCompPhantom;

            fn apply_insert(self, entity: specs::Entity, world: &specs::World, force_update: bool) {
                match self {
                    $(Self::$component_type(mut comp) => {
                        comp.pre_insert(world);
                        sync::handle_insert(comp, entity, world);
                    },)*
                    Self::Pos(comp) => {
                        sync::handle_interp_insert(comp, entity, world, force_update)
                    },
                    Self::Vel(comp) => {
                        sync::handle_interp_insert(comp, entity, world, force_update)
                    },
                    Self::Ori(comp) => {
                        sync::handle_interp_insert(comp, entity, world, force_update)
                    },
                }
            }

            fn apply_modify(self, entity: specs::Entity, world: &specs::World, force_update: bool) {
                match self {
                    $(Self::$component_type(mut comp) => {
                        comp.pre_modify(world);
                        sync::handle_modify(comp, entity, world);
                    },)*
                    Self::Pos(comp) => {
                        sync::handle_interp_modify(comp, entity, world, force_update)
                    },
                    Self::Vel(comp) => {
                        sync::handle_interp_modify(comp, entity, world, force_update)
                    },
                    Self::Ori(comp) => {
                        sync::handle_interp_modify(comp, entity, world, force_update)
                    },
                }
            }

            fn apply_remove(phantom: Self::Phantom, entity: specs::Entity, world: &specs::World) {
                match phantom {
                    $(EcsCompPhantom::$component_type(_) => {
                        sync::handle_remove::<$component_type>(entity, world);
                    },)*
                    EcsCompPhantom::Pos(_) => {
                        sync::handle_interp_remove::<comp::Pos>(entity, world)
                    },
                    EcsCompPhantom::Vel(_) => {
                        sync::handle_interp_remove::<comp::Vel>(entity, world)
                    },
                    EcsCompPhantom::Ori(_) => {
                        sync::handle_interp_remove::<comp::Ori>(entity, world)
                    },
                }
            }
        }
    }
}

use crate::synced_components::*;
crate::synced_components!(comp_packet);
