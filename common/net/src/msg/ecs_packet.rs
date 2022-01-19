use crate::sync::{self, NetSync};
use common::comp;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// This macro defines [`EcsCompPacke`]
///
/// It is meant to be passed to the `synced_components!` macro which will call
/// it with a list of components.
macro_rules! comp_packet {
    ($($component_name:ident: $component_type:ident,)*) => {

        // `sum_type!` will automatically derive From<T> for EcsCompPacket
        // for each variant EcsCompPacket::T(T).
        sum_type::sum_type! {
            #[derive(Clone, Debug, Serialize, Deserialize)]
            pub enum EcsCompPacket {
                // Note: also use the component_type identifier
                // to name the enum variant that contains the component.
                $($component_type($component_type),)*
                // These aren't included in the "synced_components" because the way
                // we determine if they are changed and when to send them is different
                // from the other components.
                Pos(comp::Pos),
                Vel(comp::Vel),
                Ori(comp::Ori),
            }
        }

        // `sum_type!` will automatically derive From<PhantomData<T>> for EcsCompPhantom
        // for each variant EcsCompPhantom::T(PhantomData<T>).
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

// Import all the component types so they will be available when expanding the
// macro below.
use crate::synced_components::*;
// Pass `comp_packet!` macro to this "x macro" which will invoke it with a list
// of components. This will declare the types defined in the macro above.
crate::synced_components!(comp_packet);
