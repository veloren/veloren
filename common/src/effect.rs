use crate::{combat, comp};
use serde::{Deserialize, Serialize};

/// An effect that may be applied to an entity
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Health(comp::HealthChange),
    Poise(f32),
    Damage(combat::Damage),
    Buff(BuffEffect),
}

/// A buff that may be applied to an entity
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuffEffect {
    pub kind: comp::BuffKind,
    pub data: comp::BuffData,
    pub cat_ids: Vec<comp::BuffCategory>,
}

impl Effect {
    pub fn info(&self) -> String {
        match self {
            Effect::Health(c) => format!("{:+} health", c.amount),
            Effect::Poise(p) => format!("{:+} poise", p),
            Effect::Damage(d) => format!("{:+}", d.value),
            Effect::Buff(e) => format!("{:?} buff", e),
        }
    }

    pub fn is_harm(&self) -> bool {
        match self {
            Effect::Health(c) => c.amount < 0.0,
            Effect::Poise(p) => *p < 0.0,
            Effect::Damage(_) => true,
            Effect::Buff(e) => !e.kind.is_buff(),
        }
    }

    pub fn modify_strength(&mut self, modifier: f32) {
        match self {
            Effect::Health(change) => {
                change.amount *= modifier;
            },
            Effect::Poise(poise) => {
                *poise *= modifier;
            },
            Effect::Damage(damage) => {
                damage.interpolate_damage(modifier, 0.0);
            },
            Effect::Buff(effect) => {
                effect.data.strength *= modifier;
            },
        }
    }
}
