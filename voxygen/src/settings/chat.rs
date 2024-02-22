use crate::hud::ChatTab;
use common::{
    comp::{ChatMsg, ChatType},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_CHAT_TABS: usize = 5;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatFilter {
    //messages
    pub message_all: bool,
    pub message_world: bool,
    pub message_region: bool,
    pub message_say: bool,
    pub message_group: bool,
    pub message_faction: bool,
    //activity (login/logout)
    pub activity_all: bool,
    pub activity_group: bool,
    //deaths
    pub death_all: bool,
    pub death_group: bool,
}
impl ChatFilter {
    pub fn satisfies(&self, chat_msg: &ChatMsg, group_members: &HashSet<&Uid>) -> bool {
        match &chat_msg.chat_type {
            ChatType::Online(u) | ChatType::Offline(u) => {
                self.activity_all || (self.activity_group && group_members.contains(u))
            },
            ChatType::CommandInfo | ChatType::CommandError => true,
            ChatType::Kill(_, u) => self.death_all || self.death_group && group_members.contains(u),
            ChatType::GroupMeta(_) => true,   //todo
            ChatType::FactionMeta(_) => true, //todo
            ChatType::Tell(..) => true,
            ChatType::Say(_) => self.message_all || self.message_say,
            ChatType::Group(..) => self.message_all || self.message_group,
            ChatType::Faction(..) => self.message_all || self.message_faction,
            ChatType::Region(_) => self.message_all || self.message_region,
            ChatType::World(_) => self.message_all || self.message_world,
            ChatType::Npc(..) => true,
            ChatType::NpcSay(..) => true,
            ChatType::NpcTell(..) => true,
            ChatType::Meta => true,
        }
    }
}
impl Default for ChatFilter {
    fn default() -> Self {
        Self {
            message_all: true,
            message_world: true,
            message_region: true,
            message_say: true,
            message_group: true,
            message_faction: true,

            activity_all: false,
            activity_group: true,

            death_all: false,
            death_group: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatSettings {
    pub chat_opacity: f32,
    pub chat_character_name: bool,
    pub chat_tabs: Vec<ChatTab>,
    pub chat_tab_index: Option<usize>,
    pub chat_cmd_prefix: char,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            chat_opacity: 0.4,
            chat_character_name: true,
            chat_tabs: vec![ChatTab::default()],
            chat_tab_index: Some(0),
            chat_cmd_prefix: '/',
        }
    }
}
