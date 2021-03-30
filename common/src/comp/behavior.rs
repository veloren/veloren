use specs::Component;
use specs_idvs::IdvStorage;
use std::mem;

use crate::trade::SiteId;

/// # Behavior Component
/// This component allow an Entity to register one or more behavior tags.
/// These tags act as flags of what an Entity can do, or what it is doing.  
/// Behaviors Tags can be added and removed as the Entity lives, to update its
/// state when needed
#[derive(Default, Clone, Debug)]
pub struct Behavior {
    tags: Vec<BehaviorTag>,
}

/// Versatile tags attached to behaviors
#[derive(PartialEq, Clone, Debug)]
pub enum BehaviorTag {
    /// The entity is allowed to speak
    CanSpeak,
    /// The entity is able to trade
    CanTrade(Option<SiteId>),

    /// The entity is currently trading
    IsTrading,
    /// The entity has issued a trade
    IsTradingIssuer,
}

impl Behavior {
    pub fn new(behavior_tags: &[BehaviorTag]) -> Self {
        let mut behavior = Self::default();
        for tag in behavior_tags.iter() {
            behavior.add_tag(tag.clone())
        }
        behavior
    }

    /// Apply a tag to the Behavior
    pub fn add_tag(&mut self, tag: BehaviorTag) {
        if !self.has_tag(&tag) {
            self.tags.push(tag);
        }
    }

    /// Revoke a tag to the Behavior
    pub fn remove_tag(&mut self, tag: BehaviorTag) {
        if self.has_tag(&tag) {
            while let Some(position) = self
                .tags
                .iter()
                .position(|behavior_tag| behavior_tag == &tag)
            {
                self.tags.remove(position);
            }
        }
    }

    /// Check if the Behavior possess a specific tag
    pub fn has_tag(&self, tag: &BehaviorTag) -> bool {
        self.tags
            .iter()
            .any(|behavior_tag| mem::discriminant(behavior_tag) == mem::discriminant(tag))
    }

    /// Get a specific tag by variant
    pub fn get_tag(&self, tag: BehaviorTag) -> Option<&BehaviorTag> {
        self.tags
            .iter()
            .find(|behavior_tag| mem::discriminant(*behavior_tag) == mem::discriminant(&tag))
    }
}

impl Component for Behavior {
    type Storage = IdvStorage<Self>;
}
