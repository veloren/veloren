use common::{
    comp::{Item, item::ItemDef},
    resources::Time,
    rtsim::{Actor, QuestId, SiteId},
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::sync::{
    Arc,
    atomic::{AtomicU8, AtomicU64, Ordering},
};

/// The easiest way to think about quests is as a virtual Jira board.
///
/// This type represents the board. In effect, it is a big database of active
/// and resolved quests. Quests are not, by themselves, 'active' participants in
/// the world. They are informal contracts, and it is up to the NPCs and players
/// that interact with them to drive them forward.
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
    /// Register a new quest ID. It can be defined later with
    /// [`Quests::create`].
    ///
    /// Critically, this function works in a shared + concurrent context, which
    /// allows us to run it in parallel within the NPC AI code.
    pub fn register(&self) -> QuestId { QuestId(self.id_counter.fetch_add(1, Ordering::Relaxed)) }

    pub fn create(&mut self, id: QuestId, quest: Quest) { self.quests.entry(id).or_insert(quest); }

    pub fn get(&self, id: QuestId) -> Option<&Quest> { self.quests.get(&id) }

    /// Resolve a quest. This can only be done once: all future attempts will
    /// fail. On success, the deposit can be returned.
    ///
    /// This function should only be invoked
    pub fn resolve(&self, id: QuestId, res: bool) -> Result<Option<(Arc<ItemDef>, u32)>, ()> {
        let quest = self.quests.get(&id).ok_or(())?;
        quest
            .res
            .0
            .compare_exchange(
                0,
                if res { 2 } else { 1 },
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .map(|_| quest.deposit.clone())
            .map_err(|_| ())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Quest {
    // The actor responsible for arbitrating over the quest.
    // Only they can decide if a quest has been completed.
    // In the future, this can be extended to include factions.
    pub arbiter: Actor,

    // A machine-intelligible description of the quest
    pub kind: QuestKind,

    // An item held in deposit. When the quest is resolved, it is returned to the arbiter (usually
    // to pass to the quest completer)
    pub deposit: Option<(Arc<ItemDef>, u32)>,

    // When the quest must be completed by
    pub timeout: Option<Time>,

    // The only aspect of the quest that mutates over time. Allows
    pub res: QuestResolution,
}

// 0 = unresolved, 1 = fail, 2.. = success
#[derive(Default, Serialize, Deserialize)]
pub struct QuestResolution(AtomicU8);

impl Clone for QuestResolution {
    fn clone(&self) -> Self {
        // This isn't strictly kosher in a multi-threaded context, but we assume that
        // clones only happen on the main thread when we don't care about
        // synchronisation
        Self(AtomicU8::new(self.0.load(Ordering::Relaxed)))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum QuestKind {
    Escort {
        escortee: Actor,
        escorter: Actor,
        to: SiteId,
    },
}

impl Quest {
    pub fn escort(escortee: Actor, escorter: Actor, to: SiteId) -> Self {
        Self {
            arbiter: escortee,
            kind: QuestKind::Escort {
                escortee,
                escorter,
                to,
            },
            deposit: None,
            timeout: None,
            res: QuestResolution(AtomicU8::new(0)),
        }
    }

    pub fn with_deposit(mut self, item: Arc<ItemDef>, amount: u32) -> Self {
        self.deposit = Some((item, amount));
        self
    }
}
