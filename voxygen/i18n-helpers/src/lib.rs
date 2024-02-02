#![feature(let_chains)]
use std::borrow::Cow;

use common::{
    comp::{
        chat::{KillSource, KillType},
        BuffKind, ChatMsg, ChatType, Content,
    },
    uid::Uid,
};
use common_net::msg::{ChatTypeContext, PlayerInfo};
use i18n::Localization;

pub fn localize_chat_message(
    msg: ChatMsg,
    lookup_fn: impl Fn(&ChatMsg) -> ChatTypeContext,
    localization: &Localization,
    show_char_name: bool,
) -> (ChatType<String>, String) {
    let info = lookup_fn(&msg);

    let name_format_or_you = |you, uid: &Uid| match info.player_alias.get(uid).cloned() {
        Some(pi) => insert_alias(you && info.you == *uid, pi, localization),
        None => info
            .entity_name
            .get(uid)
            .cloned()
            .expect("client didn't proved enough info"),
    };

    let name_format = |uid: &Uid| name_format_or_you(false, uid);

    // FIXME: this shouldn't pass review!
    let gender_str = |uid: &Uid| "he".to_owned();

    let message_format = |from: &Uid, content: &Content, group: Option<&String>| {
        let alias = name_format_or_you(true, from);
        let name = if let Some(pi) = info.player_alias.get(from).cloned() && show_char_name {
            pi.character.map(|c| c.name)
        } else {
            None
        };
        let message = localization.get_content(content);
        match (group, name) {
            (Some(group), None) => format!("({group}) [{alias}]: {message}"),
            (Some(group), Some(name)) => format!("({group}) [{alias}] {name}: {message}"),
            (None, None) => format!("[{alias}]: {message}"),
            (None, Some(name)) => format!("[{alias}] {name}: {message}"),
        }
    };

    let new_msg = match &msg.chat_type {
        ChatType::Online(uid) => localization
            .get_msg_ctx("hud-chat-online_msg", &i18n::fluent_args! {
                "user_gender" => gender_str(uid),
                "name" => name_format(uid),
            })
            .into_owned(),
        ChatType::Offline(uid) => localization
            .get_msg_ctx("hud-chat-offline_msg", &i18n::fluent_args! {
                "user_gender" => gender_str(uid),
                "name" => name_format(uid),
            })
            .into_owned(),
        ChatType::CommandError
        | ChatType::CommandInfo
        | ChatType::FactionMeta(_)
        | ChatType::GroupMeta(_) => localization.get_content(msg.content()),
        ChatType::Tell(from, to) => {
            let from_alias = name_format(from);
            let to_alias = name_format(to);
            // TODO: internationalise
            if *from == info.you {
                format!(
                    "To [{to_alias}]: {}",
                    localization.get_content(msg.content())
                )
            } else {
                format!(
                    "From [{from_alias}]: {}",
                    localization.get_content(msg.content())
                )
            }
        },
        ChatType::Say(uid) | ChatType::Region(uid) | ChatType::World(uid) => {
            message_format(uid, msg.content(), None)
        },
        ChatType::Group(uid, descriptor) | ChatType::Faction(uid, descriptor) => {
            message_format(uid, msg.content(), Some(descriptor))
        },
        // NPCs can't talk. Should be filtered by hud/mod.rs for voxygen and
        // should be filtered by server (due to not having a Pos) for chat-cli
        ChatType::Npc(uid) | ChatType::NpcSay(uid) => message_format(uid, msg.content(), None),
        ChatType::NpcTell(from, to) => {
            let from_alias = name_format(from);
            let to_alias = name_format(to);
            // TODO: internationalise
            if *from == info.you {
                format!(
                    "To [{to_alias}]: {}",
                    localization.get_content(msg.content())
                )
            } else {
                format!(
                    "From [{from_alias}]: {}",
                    localization.get_content(msg.content())
                )
            }
        },
        ChatType::Meta => localization.get_content(msg.content()),
        ChatType::Kill(kill_source, victim) => {
            localize_kill_message(kill_source, victim, name_format, gender_str, localization)
        },
    };

    (msg.chat_type, new_msg)
}

fn localize_kill_message(
    kill_source: &KillSource,
    victim: &Uid,
    name_format: impl Fn(&Uid) -> String,
    gender_str: impl Fn(&Uid) -> String,
    localization: &Localization,
) -> String {
    match kill_source {
        // Buff deaths
        KillSource::Player(attacker, KillType::Buff(buff_kind)) => {
            let buff_ident = get_buff_ident(*buff_kind);

            let s = localization
                .get_attr_ctx(
                    "hud-chat-died_of_pvp_buff_msg",
                    buff_ident,
                    &i18n::fluent_args! {
                        "victim" => name_format(victim),
                        "victim_gender" => gender_str(victim),
                        "attacker" => name_format(attacker),
                        "attacker_gender" => gender_str(attacker),
                    },
                )
                .into_owned();
            Cow::Owned(s)
        },
        KillSource::NonPlayer(attacker_name, KillType::Buff(buff_kind)) => {
            let buff_ident = get_buff_ident(*buff_kind);

            let s = localization
                .get_attr_ctx(
                    "hud-chat-died_of_npc_buff_msg",
                    buff_ident,
                    &i18n::fluent_args! {
                        "victim" => name_format(victim),
                        "victim_gender" => gender_str(victim),
                        "attacker" => attacker_name,
                    },
                )
                .into_owned();
            Cow::Owned(s)
        },
        KillSource::NonExistent(KillType::Buff(buff_kind)) => {
            let buff_ident = get_buff_ident(*buff_kind);

            let s = localization
                .get_attr_ctx(
                    "hud-chat-died_of_buff_nonexistent_msg",
                    buff_ident,
                    &i18n::fluent_args! {
                        "victim" => name_format(victim),
                        "victim_gender" => gender_str(victim),
                    },
                )
                .into_owned();
            Cow::Owned(s)
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
            localization.get_msg_ctx(key, &i18n::fluent_args! {
                "victim" => name_format(victim),
                "victim_gender" => gender_str(victim),
                "attacker" => name_format(attacker),
                "attacker_gender" => gender_str(attacker),
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
            localization.get_msg_ctx(key, &i18n::fluent_args! {
                "victim" => name_format(victim),
                "victim_gender" => gender_str(victim),
                "attacker" => attacker_name,
            })
        },
        // Other deaths
        KillSource::FallDamage => {
            localization.get_msg_ctx("hud-chat-fall_kill_msg", &i18n::fluent_args! {
                "name" => name_format(victim),
                "victim_gender" => gender_str(victim),
            })
        },
        KillSource::Suicide => {
            localization.get_msg_ctx("hud-chat-suicide_msg", &i18n::fluent_args! {
                "name" => name_format(victim),
                "victim_gender" => gender_str(victim),
            })
        },
        KillSource::NonExistent(_) | KillSource::Other => {
            localization.get_msg_ctx("hud-chat-default_death_msg", &i18n::fluent_args! {
                "name" => name_format(victim),
                "victim_gender" => gender_str(victim),
            })
        },
    }
    .into_owned()
}

// determine attr for `hud-chat-died-of-buff`
fn get_buff_ident(buff: BuffKind) -> &'static str {
    match buff {
        BuffKind::Burning => "burning",
        BuffKind::Bleeding => "bleeding",
        BuffKind::Cursed => "curse",
        BuffKind::Crippled => "crippled",
        BuffKind::Frozen => "frozen",
        BuffKind::Regeneration
        | BuffKind::Saturation
        | BuffKind::Potion
        | BuffKind::Agility
        | BuffKind::CampfireHeal
        | BuffKind::EnergyRegen
        | BuffKind::IncreaseMaxEnergy
        | BuffKind::IncreaseMaxHealth
        | BuffKind::Invulnerability
        | BuffKind::ProtectingWard
        | BuffKind::Frenzied
        | BuffKind::Hastened
        | BuffKind::Fortitude
        | BuffKind::Reckless
        | BuffKind::Flame
        | BuffKind::Frigid
        | BuffKind::Lifesteal
        // | BuffKind::SalamanderAspect
        | BuffKind::ImminentCritical
        | BuffKind::Fury
        | BuffKind::Sunderer
        | BuffKind::Defiance
        | BuffKind::Bloodfeast
        | BuffKind::Berserk => {
            tracing::error!("Player was killed by a positive buff!");
            "mysterious"
        },
        BuffKind::Wet
        | BuffKind::Ensnared
        | BuffKind::Poisoned
        | BuffKind::Parried
        | BuffKind::PotionSickness
        | BuffKind::Polymorphed
        | BuffKind::Heatstroke => {
            tracing::error!("Player was killed by a debuff that doesn't do damage!");
            "mysterious"
        },
    }
}

fn insert_alias(you: bool, info: PlayerInfo, localization: &Localization) -> String {
    // FIXME: this should take gender into account
    const YOU: &str = "hud-chat-you";
    // Leave space for a mod badge icon.
    const MOD_SPACING: &str = "      ";
    match (info.is_moderator, you) {
        (false, false) => info.player_alias,
        (false, true) => localization.get_msg(YOU).to_string(),
        (true, false) => format!("{}{}", MOD_SPACING, info.player_alias),
        (true, true) => format!("{}{}", MOD_SPACING, &localization.get_msg(YOU)),
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused)] use super::*;
    use common::comp::{
        inventory::item::{all_items_expect, ItemDesc, ItemI18n},
        Content,
    };
    use i18n::LocalizationHandle;

    // item::tests::ensure_item_localization tests that we have Content for
    // each item. This tests that we actually have at least English translation
    // for this Content.
    #[test]
    fn test_item_text() {
        let manifest = ItemI18n::new_expect();
        let localization = LocalizationHandle::load_expect("en").read();
        let items = all_items_expect();

        for item in items {
            let (name, desc) = item.i18n(&manifest);

            // check i18n for item name
            let Content::Key(key) = name else {
                panic!("name is expected to be Key, please fix the test");
            };
            localization.try_msg(&key).unwrap_or_else(|| {
                panic!("'{key}' name doesn't have i18n");
            });

            // check i18n for item desc
            let Content::Attr(key, attr) = desc else {
                panic!("desc is expected to be Attr, please fix the test");
            };
            localization.try_attr(&key, &attr).unwrap_or_else(|| {
                panic!("'{key}' description doesn't have i18n");
            });
        }
    }
}
