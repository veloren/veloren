use specs::Component;
use specs_idvs::IdvStorage;

use crate::trade::SiteId;

bitflags! {
    #[derive(Default)]
    pub struct BehaviorFlag: u8 {
        const CAN_SPEAK         = 0b00000001;
        const CAN_TRADE         = 0b00000010;
        const IS_TRADING        = 0b00000100;
        const IS_TRADING_ISSUER = 0b00001000;
    }
}

/// # Behavior Component
/// This component allow an Entity to register one or more behavior tags.
/// These tags act as flags of what an Entity can do, or what it is doing.  
/// Behaviors Tags can be added and removed as the Entity lives, to update its
/// state when needed
#[derive(Default, Copy, Clone, Debug)]
pub struct Behavior {
    pub flags: BehaviorFlag,
    pub trade_site: Option<SiteId>,
}

impl From<BehaviorFlag> for Behavior {
    fn from(flags: BehaviorFlag) -> Self {
        Behavior {
            flags,
            trade_site: None,
        }
    }
}

impl Behavior {
    pub fn set(&mut self, flags: BehaviorFlag) { self.flags.set(flags, true) }

    pub fn unset(&mut self, flags: BehaviorFlag) { self.flags.set(flags, false) }

    pub fn has(&self, flags: BehaviorFlag) -> bool { self.flags.contains(flags) }
}

impl Component for Behavior {
    type Storage = IdvStorage<Self>;
}
