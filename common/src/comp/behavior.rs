use crate::trade::SiteId;

bitflags::bitflags! {
    #[derive(Default)]
    pub struct BehaviorCapability: u8 {
        const SPEAK = 0b00000001;
        const TRADE = 0b00000010;
    }
}
bitflags::bitflags! {
    #[derive(Default)]
    pub struct BehaviorState: u8 {
        const TRADING        = 0b00000001;
        const TRADING_ISSUER = 0b00000010;
    }
}

/// # Behavior Component
/// This component allow an Entity to register one or more behavior tags.
/// These tags act as flags of what an Entity can do, or what it is doing.  
/// Behaviors Tags can be added and removed as the Entity lives, to update its
/// state when needed
#[derive(Default, Copy, Clone, Debug)]
pub struct Behavior {
    capabilities: BehaviorCapability,
    state: BehaviorState,
    pub trade_site: Option<SiteId>,
}

impl From<BehaviorCapability> for Behavior {
    fn from(capabilities: BehaviorCapability) -> Self {
        Behavior {
            capabilities,
            state: BehaviorState::default(),
            trade_site: None,
        }
    }
}

impl Behavior {
    /// Set capabilities to the Behavior
    pub fn allow(&mut self, capabilities: BehaviorCapability) {
        self.capabilities.set(capabilities, true)
    }

    /// Unset capabilities to the Behavior
    pub fn deny(&mut self, capabilities: BehaviorCapability) {
        self.capabilities.set(capabilities, false)
    }

    /// Check if the Behavior is able to do something
    pub fn can(&self, capabilities: BehaviorCapability) -> bool {
        self.capabilities.contains(capabilities)
    }

    /// Set a state to the Behavior
    pub fn set(&mut self, state: BehaviorState) { self.state.set(state, true) }

    /// Unset a state to the Behavior
    pub fn unset(&mut self, state: BehaviorState) { self.state.set(state, false) }

    /// Check if the Behavior has a specific state
    pub fn is(&self, state: BehaviorState) -> bool { self.state.contains(state) }
}

#[cfg(test)]
mod tests {
    use super::{Behavior, BehaviorCapability, BehaviorState};

    /// Test to verify that Behavior is working correctly at its most basic
    /// usages
    #[test]
    pub fn basic() {
        let mut b = Behavior::default();
        // test capabilities
        assert!(!b.can(BehaviorCapability::SPEAK));
        b.allow(BehaviorCapability::SPEAK);
        assert!(b.can(BehaviorCapability::SPEAK));
        b.deny(BehaviorCapability::SPEAK);
        assert!(!b.can(BehaviorCapability::SPEAK));
        // test states
        assert!(!b.is(BehaviorState::TRADING));
        b.set(BehaviorState::TRADING);
        assert!(b.is(BehaviorState::TRADING));
        b.unset(BehaviorState::TRADING);
        assert!(!b.is(BehaviorState::TRADING));
        // test `from`
        let b = Behavior::from(BehaviorCapability::SPEAK | BehaviorCapability::TRADE);
        assert!(b.can(BehaviorCapability::SPEAK));
        assert!(b.can(BehaviorCapability::TRADE));
        assert!(b.can(BehaviorCapability::SPEAK | BehaviorCapability::TRADE));
    }
}
