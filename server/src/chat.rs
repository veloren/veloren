use chrono::{DateTime, Utc};
use common::{
    comp,
    comp::{chat::KillType, ChatType, Content, Group, Player, UnresolvedChatMsg},
    uid::IdMaps,
    uuid::Uuid,
};
use serde::{Deserialize, Serialize};
use specs::{Join, World, WorldExt};
use std::{collections::VecDeque, ops::Sub, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{info_span, Instrument};

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    uuid: Uuid,
    alias: String,
}

/// Enum representing death reasons
///
/// All variants should be strictly typed, no string content.
#[derive(Clone, Serialize, Deserialize)]
pub enum KillSource {
    Player(PlayerInfo, KillType),
    NonPlayer(String, KillType),
    NonExistent(KillType),
    FallDamage,
    Suicide,
    Other,
}

#[derive(Clone, Serialize, Deserialize)]
/// partially mapped to common::comp::ChatMsg
pub enum ChatParties {
    Online(PlayerInfo),
    Offline(PlayerInfo),
    CommandInfo(PlayerInfo),
    CommandError(PlayerInfo),
    Kill(KillSource, PlayerInfo),
    GroupMeta(Vec<PlayerInfo>),
    Group(PlayerInfo, Vec<PlayerInfo>),
    Tell(PlayerInfo, PlayerInfo),
    Say(PlayerInfo),
    FactionMeta(String),
    Faction(PlayerInfo, String),
    Region(PlayerInfo),
    World(PlayerInfo),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub time: DateTime<Utc>,
    pub parties: ChatParties,
    pub content: Content,
}

type MessagesStore = Arc<Mutex<VecDeque<ChatMessage>>>;

/// The chat cache gets it data from the gameserver and will keep it for some
/// time It will be made available for its consumers, the REST Api
#[derive(Clone)]
pub struct ChatCache {
    pub messages: MessagesStore,
}

/// Will internally run on tokio and take stress from main loop
struct ChatForwarder {
    chat_r: tokio::sync::mpsc::Receiver<ChatMessage>,
    messages: MessagesStore,
    keep_duration: chrono::Duration,
}

pub struct ChatExporter {
    chat_s: tokio::sync::mpsc::Sender<ChatMessage>,
}

impl ChatMessage {
    fn new(chatmsg: &UnresolvedChatMsg, parties: ChatParties) -> Self {
        ChatMessage {
            time: Utc::now(),
            content: chatmsg.content().clone(),
            parties,
        }
    }
}

impl ChatExporter {
    pub fn generate(chatmsg: &UnresolvedChatMsg, ecs: &World) -> Option<ChatMessage> {
        let id_maps = ecs.read_resource::<IdMaps>();
        let players = ecs.read_storage::<Player>();
        let player_info_from_uid = |uid| {
            id_maps
                .uid_entity(uid)
                .and_then(|entry| players.get(entry))
                .map(|player| PlayerInfo {
                    alias: player.alias.clone(),
                    uuid: player.uuid(),
                })
        };
        let group_members_from_group = |g| -> Vec<_> {
            let groups = ecs.read_storage::<Group>();
            (&players, &groups)
                .join()
                .filter_map(|(player, group)| {
                    if g == group {
                        Some(PlayerInfo {
                            alias: player.alias.clone(),
                            uuid: player.uuid(),
                        })
                    } else {
                        None
                    }
                })
                .collect()
        };

        match &chatmsg.chat_type {
            ChatType::Offline(from) => {
                if let Some(player_info) = player_info_from_uid(*from) {
                    return Some(ChatMessage::new(chatmsg, ChatParties::Offline(player_info)));
                }
            },
            ChatType::Online(from) => {
                if let Some(player_info) = player_info_from_uid(*from) {
                    return Some(ChatMessage::new(chatmsg, ChatParties::Online(player_info)));
                }
            },
            ChatType::Region(from) => {
                if let Some(player_info) = player_info_from_uid(*from) {
                    return Some(ChatMessage::new(chatmsg, ChatParties::Region(player_info)));
                }
            },
            ChatType::World(from) => {
                if let Some(player_info) = player_info_from_uid(*from) {
                    return Some(ChatMessage::new(chatmsg, ChatParties::World(player_info)));
                }
            },
            ChatType::Say(from) => {
                if let Some(player_info) = player_info_from_uid(*from) {
                    return Some(ChatMessage::new(chatmsg, ChatParties::Say(player_info)));
                }
            },
            ChatType::Tell(from, to) => {
                if let (Some(from_player_info), Some(to_player_info)) =
                    (player_info_from_uid(*from), player_info_from_uid(*to))
                {
                    return Some(ChatMessage::new(
                        chatmsg,
                        ChatParties::Tell(from_player_info, to_player_info),
                    ));
                }
            },
            ChatType::Kill(kill_source, from) => {
                let kill_source = match kill_source.clone() {
                    comp::chat::KillSource::Player(uid, t) => {
                        if let Some(player_info) = player_info_from_uid(uid) {
                            KillSource::Player(player_info, t)
                        } else {
                            return None;
                        }
                    },
                    comp::chat::KillSource::NonPlayer(str, t) => KillSource::NonPlayer(str, t),
                    comp::chat::KillSource::NonExistent(t) => KillSource::NonExistent(t),
                    comp::chat::KillSource::FallDamage => KillSource::FallDamage,
                    comp::chat::KillSource::Suicide => KillSource::Suicide,
                    comp::chat::KillSource::Other => KillSource::Other,
                };
                if let Some(player_info) = player_info_from_uid(*from) {
                    return Some(ChatMessage::new(
                        chatmsg,
                        ChatParties::Kill(kill_source, player_info),
                    ));
                }
            },
            ChatType::FactionMeta(s) => {
                return Some(ChatMessage::new(
                    chatmsg,
                    ChatParties::FactionMeta(s.clone()),
                ));
            },
            ChatType::Faction(from, s) => {
                if let Some(player_info) = player_info_from_uid(*from) {
                    return Some(ChatMessage::new(
                        chatmsg,
                        ChatParties::Faction(player_info, s.clone()),
                    ));
                }
            },
            ChatType::GroupMeta(g) => {
                let members = group_members_from_group(g);
                return Some(ChatMessage::new(chatmsg, ChatParties::GroupMeta(members)));
            },
            ChatType::Group(from, g) => {
                let members = group_members_from_group(g);
                if let Some(player_info) = player_info_from_uid(*from) {
                    return Some(ChatMessage::new(
                        chatmsg,
                        ChatParties::Group(player_info, members),
                    ));
                }
            },
            _ => (),
        };

        None
    }

    pub fn send(&self, msg: ChatMessage) {
        if let Err(e) = self.chat_s.blocking_send(msg) {
            tracing::warn!(
                ?e,
                "could not export chat message. the tokio sender seems to be broken"
            );
        }
    }
}

impl ChatForwarder {
    async fn run(mut self) {
        while let Some(msg) = self.chat_r.recv().await {
            let drop_older_than = msg.time.sub(self.keep_duration);
            let mut messages = self.messages.lock().await;
            while let Some(msg) = messages.front()
                && msg.time < drop_older_than
            {
                messages.pop_front();
            }
            messages.push_back(msg);
            const MAX_CACHE_MESSAGES: usize = 10_000; // in case we have a short spam of many many messages, we dont want to keep the capacity forever
            if messages.capacity() > messages.len() + MAX_CACHE_MESSAGES {
                let msg_count = messages.len();
                tracing::debug!(?msg_count, "shrinking cache");
                messages.shrink_to_fit();
            }
        }
    }
}

impl ChatCache {
    pub fn new(keep_duration: Duration, runtime: &tokio::runtime::Runtime) -> (Self, ChatExporter) {
        const BUFFER_SIZE: usize = 1_000;
        let (chat_s, chat_r) = tokio::sync::mpsc::channel(BUFFER_SIZE);
        let messages: Arc<Mutex<VecDeque<ChatMessage>>> = Default::default();
        let messages_clone = Arc::clone(&messages);
        let keep_duration = chrono::Duration::from_std(keep_duration).unwrap();

        let worker = ChatForwarder {
            keep_duration,
            chat_r,
            messages: messages_clone,
        };

        runtime.spawn(worker.run().instrument(info_span!("chat_forwarder")));

        (Self { messages }, ChatExporter { chat_s })
    }
}
