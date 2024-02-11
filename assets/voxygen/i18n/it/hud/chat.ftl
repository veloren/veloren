## Player events, $user_gender should be available

hud-chat-online_msg = { $user_gender ->
    [she] [{ $name }] si è connessa
    *[he] [{ $name }] si è connesso
}
hud-chat-offline_msg = { $user_gender ->
    [she] [{ $name }] si è sconnessa
    *[he] [{ $name }] si è sconnesso
}
hud-chat-goodbye = Arrivederci!
hud-chat-connection_lost = { $user_gender ->
    [she] Connessione persa. Verrai scollegata tra { $time } secondi.
    *[he] Connessione persa. Verrai scollegato tra { $time } secondi.
}

## Player /tell messages, $user_gender should be available

hud-chat-tell-to = Per [{ $alias }]: { $msg }
hud-chat-tell-from = Da [{ $alias }]: { $msg }


# Npc /tell messages, no gender info, sadly

hud-chat-tell-to-npc = Per [{ $alias }]: { $msg }
hud-chat-tell-from-npc = Da [{ $alias }]: { $msg }

# Generic messages

hud-chat-message = [{ $alias }]: { $msg }
hud-chat-message-with-name = [{ $alias }] { $name }: { $msg }
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }

## PvP Buff deaths, both $attacker_gender and $victim_gender are available

hud-chat-died_of_pvp_buff_msg =
    .burning = { $victim_gender ->
    [she] [{ $victim }] è morta bruciata da [{ $attacker }]
    *[he] [{ $victim }] è morto bruciato da [{ $attacker }]
}
    .bleeding = { $victim_gender ->
    [she] [{ $victim }] è morta dissanguata da [{ $attacker }]
    *[he] [{ $victim }] è morto dissanguato da [{ $attacker }]
}
    .curse = { $victim_gender ->
    [she] [{ $victim }] è morta per una maledizione di [{ $attacker }]
    *[he] [{ $victim }] è morto per una maledizione di [{ $attacker }]
}
    .crippled = { $victim_gender ->
    [she] [{ $victim }] è morta mutilata da [{ $attacker }]
    *[he] [{ $victim }] è morto mutilato da [{ $attacker }]
}
    .frozen = { $victim_gender ->
    [she] [{ $victim }] è morta congelata da [{ $attacker }]
    *[he] [{ $victim }] è morto congelato da [{ $attacker }]
}
    .mysterious = { $victim_gender ->
    [she] [{ $victim }] è morta per cause misteriose a causa di [{ $attacker }]
    *[he] [{ $victim }] è morto per cause misteriose a causa di [{ $attacker }]
}

## PvE Buff deaths, only $victim_gender is available

hud-chat-died_of_npc_buff_msg =
    .burning = { $victim_gender ->
    [she] [{ $victim }] è morta bruciata da { $attacker }
    *[he] [{ $victim }] è morto bruciato da { $attacker }
}
    .bleeding = { $victim_gender ->
    [she] [{ $victim }] è morta dissanguata da { $attacker }
    *[he] [{ $victim }] è morto dissanguato da { $attacker }
}
    .curse = { $victim_gender ->
    [she] [{ $victim }] è morta per una maledizione di { $attacker }
    *[he] [{ $victim }] è morto per una maledizione di { $attacker }
}
    .crippled = { $victim_gender ->
    [she] [{ $victim }] è morta per mutilazioni subite da { $attacker }
    *[he] [{ $victim }] è morto per mutilazioni subite da { $attacker }
}
    .frozen = { $victim_gender ->
    [she] [{ $victim }] è morta congelata da { $attacker }
    *[he] [{ $victim }] è morto congelato da { $attacker }
}
    .mysterious = { $victim_gender ->
    [she] [{ $victim }] è stata uccisa in circostanze misteriose da { $attacker }
    *[he] [{ $victim }] è stato ucciso in circostanze misteriose da { $attacker }
}

## Random Buff deaths, only $victim_gender is available

hud-chat-died_of_buff_nonexistent_msg =
    .burning = { $victim_gender ->
    [she] [{ $victim }] è morta bruciata
    *[he] [{ $victim }] è morto bruciato
}
    .bleeding = { $victim_gender ->
    [she] [{ $victim }] è morta dissanguata
    *[he] [{ $victim }] è morto dissanguato
}
    .curse = { $victim_gender ->
    [she] [{ $victim }] è morta per una maledizione
    *[he] [{ $victim }] è morto per una maledizione
}
    .crippled = { $victim_gender ->
    [she] [{ $victim }] è morta per mutilazioni subite
    *[he] [{ $victim }] è morto per mutilazioni subite
}
    .frozen = { $victim_gender ->
    [she] [{ $victim }] è morta congelata
    *[he] [{ $victim }] è morto congelato
}
    .mysterious = { $victim_gender ->
    [she] [{ $victim }] è morta in circostanze misteriose
    *[he] [{ $victim }] è morto in circostanze misteriose
}

## Other PvP deaths, both $attacker_gender and $victim_gender are available

hud-chat-pvp_melee_kill_msg = [{ $attacker }] ha sconfitto [{ $victim }]
hud-chat-pvp_ranged_kill_msg = [{ $attacker }] ha assassinato [{ $victim }]
hud-chat-pvp_explosion_kill_msg = [{ $attacker }] ha fatto esplodere [{ $victim }]
hud-chat-pvp_energy_kill_msg = [{ $attacker }] ha ucciso [{ $victim }] con la magia
hud-chat-pvp_other_kill_msg = [{ $attacker }] ha ucciso [{ $victim }]

## Other PvE deaths, only $victim_gender is available

hud-chat-npc_melee_kill_msg = { $attacker } ha ucciso [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } ha assassinato [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } ha fatto esplodere [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } ha ucciso [{ $victim }] con la magia
hud-chat-npc_other_kill_msg = { $attacker } ha ucciso [{ $victim }]

## Other deaths, only $victim_gender is available

hud-chat-fall_kill_msg = { $victim_gender ->
    [she] [{ $name }] è caduta ed è morta
    *[he] [{ $name }] è caduto ed è morto
}
hud-chat-suicide_msg = { $victim_gender ->
    [she] [{ $name }] è morta per ferite auto inflitte
    *[he] [{ $name }] è morto per ferite auto inflitte
}
hud-chat-default_death_msg = [{ $name }] { $victim_gender ->
    [she] [{ $name }] è morta
    *[he] [{ $name }] è morto
}

## Chat utils

hud-chat-all = Tutti
hud-chat-chat_tab_hover_tooltip = Click destro per le impostazioni

## HUD Pickup message

hud-loot-pickup-msg =
    { $actor } ha raccolto { $amount ->
        [one] { $item }
       *[other] { $amount }x { $item }
    }
