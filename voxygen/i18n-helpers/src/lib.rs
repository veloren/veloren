#![feature(let_chains)]
use std::borrow::Cow;

use common::{
    comp::{
        body::Gender,
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

    let name_format_or_complex = |complex, uid: &Uid| match info.player_info.get(uid).cloned() {
        Some(pi) => {
            if complex {
                insert_alias(info.you == *uid, pi, localization)
            } else {
                pi.player_alias
            }
        },
        None => info
            .entity_name
            .get(uid)
            .cloned()
            .expect("client didn't proved enough info"),
    };

    // Some messages do suffer from complicated logic of insert_alias.
    // This includes every notification-like message, like death.
    let name_format = |uid: &Uid| name_format_or_complex(false, uid);

    // This is a hack, kind of.
    //
    // Current implementation just checks if our player is humanoid, and if so,
    // we take the body_type of its character and assume it as grammatical gender.
    //
    // In short,
    //  body_type of character
    //  -> sex of character
    //  -> gender of player.
    //  -> grammatical gender for use in messages.
    //
    // This is obviously, wrong, but it's good enough approximation, after all,
    // players do choose their characters.
    //
    // In the future, we will want special GUI where players can specify their
    // gender (and change it!), and we do want to handle more genders than just
    // male and female.
    //
    // Ideally, the system should handle following (if we exclude plurals):
    // - Female
    // - Male
    // - Neuter (or fallback Female)
    // - Neuter (or fallback Male)
    // - Intermediate (or fallback Female)
    // - Intermediate (or fallback Male)
    // and maybe more, not sure.
    //
    // What is supported by language and what is not, as well as maybe how to
    // convert genders into strings to match, should go into _manifest.ron file
    //
    // So let's say language only supports male and female, we will convert all
    // genders to these, using some fallbacks, and pass it.
    //
    // If the language can represent Female, Male and Neuter, we can pass these.
    //
    // Exact design of such a complex system is honestly up to discussion.
    let gender_str = |uid: &Uid| {
        if let Some(pi) = info.player_info.get(uid) {
            match pi.character.as_ref().and_then(|c| c.gender) {
                Some(Gender::Feminine) => "she".to_owned(),
                Some(Gender::Masculine) => "he".to_owned(),
                None => {
                    tracing::error!("We tried to get the gender, but failed");

                    "??".to_owned()
                },
            }
        } else {
            tracing::error!("We tried to get the gender of the player we can't find");

            "??".to_owned()
        }
    };

    // This is where the most fun begings.
    //
    // Unlike people, "items" can have their own gender, which is completely
    // independent of everything, including common sense.
    //
    // For example, word "masculinity" can be feminine in some languages,
    // as well as word "boy", and vice versa.
    //
    // So we can't rely on body_type, at all. And even if we did try, our
    // body_type isn't even always represents animal sex, there are some
    // animals that use body_type to represent their kind, like different
    // types of Fox ("male" fox is forest, "female" is arctic one).
    // And what about Mindflayer? They do have varied body_type, but do they
    // even have concept of gender?
    //
    // Our use case is probably less cryptic, after all we are limited by
    // mostly sentient things, but that doesn't help at all.
    //
    // Common example is word "spider", which can be feminine in one languages
    // and masculine in other, and sometimes even neuter.
    //
    // Oh, and I want to add that we are talking about grammatical genders, and
    // languages define their own grammar. There are languages that have more
    // than three grammatical genders, there are languages that don't have
    // male/female distinction and instead realy on animacy/non-animacy.
    // What has an animacy and what doesn't is for language to decide.
    // There are languages as well that mix these concepts and may have neuter,
    // female, masculine with animacy, masculine with animacy. Some languages
    // have their own scheme of things that arbitrarily picks noun-class per
    // noun.
    // Don't get me wrong. *All* languages do pick the gender for the word
    // arbitrary as I showed at the beginning, it's just some languages have
    // not just different mapping, but different gender set as well.
    //
    // The *only* option we have is fetch the gender per each name entry from
    // localization files.
    //
    // I'm not 100% sure what should be the implementation of it, but I imagine
    // that Stats::name() should be changed to include a way to reference where
    // to grab the gender associated with this name, so translation then can
    // pick right article or use right adjective/verb connected with NPC in the
    // context of the message.
    let _gender_str_npc = || "idk".to_owned();

    let message_format = |from: &Uid, content: &Content, group: Option<&String>| {
        let alias = name_format_or_complex(true, from);

        let name = if let Some(pi) = info.player_info.get(from).cloned() && show_char_name {
            pi.character.map(|c| c.name)
        } else {
            None
        };

        let message = localization.get_content(content);

        let line = match group {
            Some(group) => match name {
                Some(name) => localization.get_msg_ctx(
                    "hud-chat-message-in-group-with-name",
                    &i18n::fluent_args! {
                        "group" => group,
                        "alias" => alias,
                        "name" => name,
                        "msg" => message,
                    },
                ),
                None => {
                    localization.get_msg_ctx("hud-chat-message-in-group", &i18n::fluent_args! {
                        "group" => group,
                        "alias" => alias,
                        "msg" => message,
                    })
                },
            },
            None => match name {
                Some(name) => {
                    localization.get_msg_ctx("hud-chat-message-with-name", &i18n::fluent_args! {
                        "alias" => alias,
                        "name" => name,
                        "msg" => message,
                    })
                },
                None => localization.get_msg_ctx("hud-chat-message", &i18n::fluent_args! {
                    "alias" => alias,
                    "msg" => message,
                }),
            },
        };

        line.into_owned()
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
            // If `from` is you, it means you're writing to someone
            // and you want to see who you're writing to.
            //
            // Otherwise, someone writes to you, and you want to see
            // who is that person that's writing to you.
            let (key, person_to_show) = if info.you == *from {
                ("hud-chat-tell-to", to)
            } else {
                ("hud-chat-tell-from", from)
            };

            localization
                .get_msg_ctx(key, &i18n::fluent_args! {
                    "alias" => name_format(person_to_show),
                    "user_gender" => gender_str(person_to_show),
                })
                .into_owned()
        },
        ChatType::Say(uid) | ChatType::Region(uid) | ChatType::World(uid) => {
            message_format(uid, msg.content(), None)
        },
        ChatType::Group(uid, descriptor) | ChatType::Faction(uid, descriptor) => {
            message_format(uid, msg.content(), Some(descriptor))
        },
        ChatType::Npc(uid) | ChatType::NpcSay(uid) => message_format(uid, msg.content(), None),
        ChatType::NpcTell(from, to) => {
            // If `from` is you, it means you're writing to someone
            // and you want to see who you're writing to.
            //
            // Otherwise, someone writes to you, and you want to see
            // who is that person that's writing to you.
            //
            // Hopefully, no gendering needed, because for npc, we
            // simply don't know.
            let (key, person_to_show) = if info.you == *from {
                ("hud-chat-tell-to-npc", to)
            } else {
                ("hud-chat-tell-from-npc", from)
            };

            localization
                .get_msg_ctx(key, &i18n::fluent_args! {
                    "alias" => name_format(person_to_show),
                })
                .into_owned()
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
        // PvP deaths
        KillSource::Player(attacker, kill_type) => {
            let key = match kill_type {
                KillType::Melee => "hud-chat-pvp_melee_kill_msg",
                KillType::Projectile => "hud-chat-pvp_ranged_kill_msg",
                KillType::Explosion => "hud-chat-pvp_explosion_kill_msg",
                KillType::Energy => "hud-chat-pvp_energy_kill_msg",
                KillType::Other => "hud-chat-pvp_other_kill_msg",
                KillType::Buff(buff_kind) => {
                    return {
                        let buff_ident = get_buff_ident(*buff_kind);

                        localization
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
                            .into_owned()
                    };
                },
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
                KillType::Buff(buff_kind) => {
                    return {
                        let buff_ident = get_buff_ident(*buff_kind);

                        localization
                            .get_attr_ctx(
                                "hud-chat-died_of_npc_buff_msg",
                                buff_ident,
                                &i18n::fluent_args! {
                                    "victim" => name_format(victim),
                                    "victim_gender" => gender_str(victim),
                                    "attacker" => attacker_name,
                                },
                            )
                            .into_owned()
                    };
                },
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

// TODO: consider fetching "you" string from localization and somehow fetching
// user's gender and putting it as argument.
//
// Altenatively, which would be a better design, pass 'is_you' to
// hud-chat-message and provide $mod_spacing attribute to use.
fn insert_alias(_replace_you: bool, info: PlayerInfo, _localization: &Localization) -> String {
    // Leave space for a mod badge icon.
    const MOD_SPACING: &str = "      ";

    if info.is_moderator {
        info.player_alias
    } else {
        format!("{}{}", MOD_SPACING, info.player_alias)
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
