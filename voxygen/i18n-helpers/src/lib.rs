#![feature(let_chains)]
use common::comp::{
    chat::{KillSource, KillType},
    BuffKind, ChatMsg, ChatType,
};
use common_net::msg::{ChatTypeContext, PlayerInfo};
use i18n::Localization;

pub fn localize_chat_message(
    mut msg: ChatMsg,
    lookup_fn: impl Fn(&ChatMsg) -> ChatTypeContext,
    localisation: &Localization,
    show_char_name: bool,
) -> ChatMsg {
    let info = lookup_fn(&msg);

    let name_format = |uid: &common::uid::Uid| match info.player_alias.get(uid).cloned() {
        Some(pi) => insert_alias(info.you == *uid, pi, localisation),
        None => info
            .entity_name
            .get(uid)
            .cloned()
            .expect("client didn't proved enough info"),
    };

    let message_format = |from: &common::uid::Uid, message: &str, group: Option<&String>| {
        let alias = name_format(from);
        let name = if let Some(pi) = info.player_alias.get(from).cloned() && show_char_name {
            pi.character.map(|c| c.name)
        } else {
            None
        };
        match (group, name) {
            (Some(group), None) => format!("({group}) [{alias}]: {message}"),
            (None, None) => format!("[{alias}]: {message}"),
            (Some(group), Some(name)) => format!("({group}) [{alias}] {name}: {message}"),
            (None, Some(name)) => format!("[{alias}] {name}: {message}"),
        }
    };

    let new_msg = match &msg.chat_type {
        ChatType::Online(uid) => localisation
            .get_msg_ctx("hud-chat-online_msg", &i18n::fluent_args! {
                "name" => name_format(uid),
            })
            .into_owned(),
        ChatType::Offline(uid) => localisation
            .get_msg_ctx("hud-chat-offline_msg", &i18n::fluent_args! {
                "name" => name_format(uid
                ),
            })
            .into_owned(),
        ChatType::CommandError => msg.message.to_string(),
        ChatType::CommandInfo => msg.message.to_string(),
        ChatType::FactionMeta(_) => msg.message.to_string(),
        ChatType::GroupMeta(_) => msg.message.to_string(),
        ChatType::Tell(from, to) => {
            let from_alias = name_format(from);
            let to_alias = name_format(to);
            // TODO: internationalise
            if *from == info.you {
                format!("To [{to_alias}]: {}", msg.message)
            } else {
                format!("From [{from_alias}]: {}", msg.message)
            }
        },
        ChatType::Say(uid) => message_format(uid, &msg.message, None),
        ChatType::Group(uid, s) => message_format(uid, &msg.message, Some(s)),
        ChatType::Faction(uid, s) => message_format(uid, &msg.message, Some(s)),
        ChatType::Region(uid) => message_format(uid, &msg.message, None),
        ChatType::World(uid) => message_format(uid, &msg.message, None),
        // NPCs can't talk. Should be filtered by hud/mod.rs for voxygen and
        // should be filtered by server (due to not having a Pos) for chat-cli
        ChatType::Npc(uid, _r) => message_format(uid, &msg.message, None),
        ChatType::NpcSay(uid, _r) => message_format(uid, &msg.message, None),
        ChatType::NpcTell(from, to, _r) => {
            let from_alias = name_format(from);
            let to_alias = name_format(to);
            // TODO: internationalise
            if *from == info.you {
                format!("To [{to_alias}]: {}", msg.message)
            } else {
                format!("From [{from_alias}]: {}", msg.message)
            }
        },
        ChatType::Meta => msg.message.to_string(),
        ChatType::Kill(kill_source, victim) => {
            let i18n_buff = |buff| match buff {
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
                | BuffKind::Hastened
                | BuffKind::Fortitude
                | BuffKind::Reckless => {
                    tracing::error!("Player was killed by a positive buff!");
                    "hud-outcome-mysterious"
                },
                BuffKind::Wet
                | BuffKind::Ensnared
                | BuffKind::Poisoned
                | BuffKind::Parried
                | BuffKind::PotionSickness
                | BuffKind::Polymorphed(_) => {
                    tracing::error!("Player was killed by a debuff that doesn't do damage!");
                    "hud-outcome-mysterious"
                },
            };

            match kill_source {
                // Buff deaths
                KillSource::Player(attacker, KillType::Buff(buff_kind)) => {
                    let i18n_buff = i18n_buff(*buff_kind);
                    let buff = localisation.get_msg(i18n_buff);

                    localisation.get_msg_ctx("hud-chat-died_of_pvp_buff_msg", &i18n::fluent_args! {
                        "victim" => name_format(victim),
                        "died_of_buff" => buff,
                        "attacker" => name_format(attacker),
                    })
                },
                KillSource::NonPlayer(attacker_name, KillType::Buff(buff_kind)) => {
                    let i18n_buff = i18n_buff(*buff_kind);
                    let buff = localisation.get_msg(i18n_buff);

                    localisation.get_msg_ctx("hud-chat-died_of_npc_buff_msg", &i18n::fluent_args! {
                        "victim" => name_format(victim),
                        "died_of_buff" => buff,
                        "attacker" => attacker_name,
                    })
                },
                KillSource::NonExistent(KillType::Buff(buff_kind)) => {
                    let i18n_buff = i18n_buff(*buff_kind);
                    let buff = localisation.get_msg(i18n_buff);

                    localisation.get_msg_ctx(
                        "hud-chat-died_of_buff_nonexistent_msg",
                        &i18n::fluent_args! {
                            "victim" => name_format(victim),
                            "died_of_buff" => buff,
                        },
                    )
                },
                // PvP deaths
                KillSource::Player(attacker, kill_type) => {
                    let key = match kill_type {
                        KillType::Melee => "hud-chat-pvp_melee_kill_msg",
                        KillType::Projectile => "hud-chat-pvp_ranged_kill_msg",
                        KillType::Explosion => "hud-chat-pvp_explosion_kill_msg",
                        KillType::Energy => "hud-chat-pvp_energy_kill_msg",
                        KillType::Other => "hud-chat-pvp_other_kill_msg",
                        &KillType::Buff(_) => unreachable!("handled above"),
                    };
                    localisation.get_msg_ctx(key, &i18n::fluent_args! {
                        "victim" => name_format(victim),
                        "attacker" => name_format(attacker),
                    })
                },
                // PvE deaths
                KillSource::NonPlayer(attacker_name, kill_type) => {
                    let key = match kill_type {
                        KillType::Melee => "hud-chat-npc_melee_kill_msg",
                        KillType::Projectile => "hud-chat-npc_ranged_kill_msg",
                        KillType::Explosion => "hud-chat-npc_explosion_kill_msg",
                        KillType::Energy => "hud-chat-npc_energy_kill_msg",
                        KillType::Other => "hud-chat-npc_other_kill_msg",
                        &KillType::Buff(_) => unreachable!("handled above"),
                    };
                    localisation.get_msg_ctx(key, &i18n::fluent_args! {
                        "victim" => name_format(victim),
                        "attacker" => attacker_name,
                    })
                },
                // Other deaths
                KillSource::Environment(environment) => {
                    localisation.get_msg_ctx("hud-chat-environment_kill_msg", &i18n::fluent_args! {
                        "name" => name_format(victim),
                        "environment" => environment,
                    })
                },
                KillSource::FallDamage => {
                    localisation.get_msg_ctx("hud-chat-fall_kill_msg", &i18n::fluent_args! {
                        "name" => name_format(victim),
                    })
                },
                KillSource::Suicide => {
                    localisation.get_msg_ctx("hud-chat-suicide_msg", &i18n::fluent_args! {
                        "name" => name_format(victim),
                    })
                },
                KillSource::NonExistent(_) | KillSource::Other => {
                    localisation.get_msg_ctx("hud-chat-default_death_msg", &i18n::fluent_args! {
                        "name" => name_format(victim),
                    })
                },
            }
            .to_string()
        },
    };

    msg.message = new_msg;
    msg
}

fn insert_alias(you: bool, info: PlayerInfo, localisation: &Localization) -> String {
    const YOU: &str = "hud-chat-you";
    // Leave space for a mod badge icon.
    const MOD_SPACING: &str = "      ";
    match (info.is_moderator, you) {
        (false, false) => info.player_alias,
        (false, true) => localisation.get_msg(YOU).to_string(),
        (true, false) => format!("{}{}", MOD_SPACING, info.player_alias),
        (true, true) => format!("{}{}", MOD_SPACING, &localisation.get_msg(YOU),),
    }
}
