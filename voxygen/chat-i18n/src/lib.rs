use common::comp::{
    chat::{KillSource, KillType},
    BuffKind, ChatMsg, ChatType,
};
use common_net::msg::{ChatTypeContext, PlayerInfo};
use i18n::Localization;
use std::collections::HashMap;

pub fn internationalisate_chat_message(
    mut msg: ChatMsg,
    lookup_fn: impl Fn(&ChatMsg) -> HashMap<&'static str, ChatTypeContext>,
    localized_strings: &Localization,
    show_char_name: bool,
) -> ChatMsg {
    if let Some(template_key) = get_chat_template_key(&msg.chat_type) {
        msg.message = localized_strings
            .get_msg_ctx(template_key, &i18n::fluent_args! {
                "attacker" => "{attacker}",
                "attacker" => "{attacker_name}",
                "name" => "{player}",
                "died_of_buff" => "{died_of_buff}",
                "victim" => "{victim}",
                "environment" => "{environment}",
            })
            .into_owned();

        if let ChatType::Kill(kill_source, _) = &msg.chat_type {
            match kill_source {
                KillSource::Player(_, KillType::Buff(buffkind))
                | KillSource::NonExistent(KillType::Buff(buffkind))
                | KillSource::NonPlayer(_, KillType::Buff(buffkind)) => {
                    msg.message = insert_killing_buff(*buffkind, localized_strings, &msg.message);
                },
                _ => {},
            }
        }
    }
    let info = lookup_fn(&msg);
    let gen_alias = |you, info: PlayerInfo| {
        let mod_str = if info.is_moderator { "MOD - " } else { "" };
        let you_str = if you { "You" } else { &info.player_alias };
        format!("{}{}", mod_str, you_str)
    };
    let message_format = |you, info: PlayerInfo, message: &str, group: Option<&String>| {
        let alias = gen_alias(you, info.clone());
        let name = if show_char_name {
            info.character.map(|c| c.name)
        } else {
            None
        };
        match (group, name) {
            (Some(group), None) => format!("({}) [{}]: {}", group, alias, message),
            (None, None) => format!("[{}]: {}", alias, message),
            (Some(group), Some(name)) => {
                format!("({}) [{}] {}: {}", group, alias, name, message)
            },
            (None, Some(name)) => format!("[{}] {}: {}", alias, name, message),
        }
    };
    if let Some(ChatTypeContext::PlayerAlias { you, info }) = info.get("from").cloned() {
        msg.message = match &msg.chat_type {
            ChatType::Say(_) => message_format(you, info, &msg.message, None),
            ChatType::Group(_, s) => message_format(you, info, &msg.message, Some(s)),
            ChatType::Faction(_, s) => message_format(you, info, &msg.message, Some(s)),
            ChatType::Region(_) => message_format(you, info, &msg.message, None),
            ChatType::World(_) => message_format(you, info, &msg.message, None),
            ChatType::NpcSay(_, _r) => message_format(you, info, &msg.message, None),
            _ => msg.message,
        };
    }
    for (name, datum) in info.into_iter() {
        let replacement = match datum {
            ChatTypeContext::PlayerAlias { you, info } => gen_alias(you, info),
            ChatTypeContext::Raw(text) => text,
        };
        msg.message = msg.message.replace(&format!("{{{}}}", name), &replacement);
    }
    msg
}

fn get_chat_template_key(chat_type: &ChatType<String>) -> Option<&str> {
    Some(match chat_type {
        ChatType::Online(_) => "hud-chat-online_msg",
        ChatType::Offline(_) => "hud-chat-offline_msg",
        ChatType::Kill(kill_source, _) => match kill_source {
            KillSource::Player(_, KillType::Buff(_)) => "hud-chat-died_of_pvp_buff_msg",
            KillSource::Player(_, KillType::Melee) => "hud-chat-pvp_melee_kill_msg",
            KillSource::Player(_, KillType::Projectile) => "hud-chat-pvp_ranged_kill_msg",
            KillSource::Player(_, KillType::Explosion) => "hud-chat-pvp_explosion_kill_msg",
            KillSource::Player(_, KillType::Energy) => "hud-chat-pvp_energy_kill_msg",
            KillSource::Player(_, KillType::Other) => "hud-chat-pvp_other_kill_msg",
            KillSource::NonExistent(KillType::Buff(_)) => "hud-chat-died_of_buff_nonexistent_msg",
            KillSource::NonPlayer(_, KillType::Buff(_)) => "hud-chat-died_of_npc_buff_msg",
            KillSource::NonPlayer(_, KillType::Melee) => "hud-chat-npc_melee_kill_msg",
            KillSource::NonPlayer(_, KillType::Projectile) => "hud-chat-npc_ranged_kill_msg",
            KillSource::NonPlayer(_, KillType::Explosion) => "hud-chat-npc_explosion_kill_msg",
            KillSource::NonPlayer(_, KillType::Energy) => "hud-chat-npc_energy_kill_msg",
            KillSource::NonPlayer(_, KillType::Other) => "hud-chat-npc_other_kill_msg",
            KillSource::Environment(_) => "hud-chat-environmental_kill_msg",
            KillSource::FallDamage => "hud-chat-fall_kill_msg",
            KillSource::Suicide => "hud-chat-suicide_msg",
            KillSource::NonExistent(_) | KillSource::Other => "hud-chat-default_death_msg",
        },
        _ => return None,
    })
}

fn insert_killing_buff(buff: BuffKind, localized_strings: &Localization, template: &str) -> String {
    let buff_outcome = match buff {
        BuffKind::Burning => "hud-outcome-burning",
        BuffKind::Bleeding => "hud-outcome-bleeding",
        BuffKind::Cursed => "hud-outcome-curse",
        BuffKind::Crippled => "hud-outcome-crippled",
        BuffKind::Frozen => "hud-outcome-frozen",
        BuffKind::Regeneration
        | BuffKind::Saturation
        | BuffKind::Potion
        | BuffKind::CampfireHeal
        | BuffKind::EnergyRegen
        | BuffKind::IncreaseMaxEnergy
        | BuffKind::IncreaseMaxHealth
        | BuffKind::Invulnerability
        | BuffKind::ProtectingWard
        | BuffKind::Frenzied
        | BuffKind::Hastened => {
            tracing::error!("Player was killed by a positive buff!");
            "hud-outcome-mysterious"
        },
        BuffKind::Wet | BuffKind::Ensnared | BuffKind::Poisoned => {
            tracing::error!("Player was killed by a debuff that doesn't do damage!");
            "hud-outcome-mysterious"
        },
    };

    template.replace("{died_of_buff}", &localized_strings.get_msg(buff_outcome))
}
