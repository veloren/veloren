use common::{
    comp::Item,
    resources::Time,
    rtsim::{Actor, QuestId, SiteId},
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default, Serialize, Deserialize)]
pub struct Quests {
    /// Because quests can be created in a multi-threaded context, we use an
    /// atomic counter to generate IDs for them. Quest insertion happens at
    /// the end of each tick. This is guarded by a utility function, so
    /// unregistered quests *shouldn't* be visible to the rest of the code.
    id_counter: AtomicU64,
    quests: HashMap<QuestId, Quest>,
}

impl Clone for Quests {
    fn clone(&self) -> Self {
        Self {
            // This isn't strictly kosher in a multi-threaded context, but we assume that clones
            // only happen on the main thread when we don't care about synchronisation
            id_counter: AtomicU64::new(self.id_counter.load(Ordering::SeqCst)),
            quests: self.quests.clone(),
        }
    }
}

impl Quests {
    pub fn register(&self) -> QuestId { QuestId(self.id_counter.fetch_add(1, Ordering::Relaxed)) }

    pub fn create(&mut self, id: QuestId, quest: Quest) { self.quests.entry(id).or_insert(quest); }

    pub fn get(&self, id: QuestId) -> Option<&Quest> { self.quests.get(&id) }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Quest {
    Escort {
        escortee: Actor,
        escorter: Actor,
        to: SiteId,
    },
}
