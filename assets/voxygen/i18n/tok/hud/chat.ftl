hud-chat-online_msg = { "[" }{ $name }] li kama lon.
hud-chat-offline_msg = { "[" }{ $name }] li weka.
hud-chat-goodbye = tawa pona!
hud-chat-tell-to = tawa jan [{ $alias }]: { $msg }
hud-chat-tell-from = tan jan [{ $alias }]: { $msg }
hud-chat-tell-to-npc = tawa jan [{ $alias }]: { $msg }
hud-chat-tell-from-npc = tan jan [{ $alias }]: { $msg }
hud-chat-message = { "[" }{ $alias }]: { $msg }
hud-chat-message-with-name = { "[" }{ $alias }] { $name }: { $msg }
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }
hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }] li moli e jan [{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }] li moli e [{ $victim }] kepeken ilo pana
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }] li moli e jan [{ $victim }] kepeken seli pakala
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] li moli e jan [{ $victim }] kepeken nasa
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] li moli e jan [{ $victim }]
hud-chat-npc_melee_kill_msg = { $attacker } li moli e jan [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } li moli e jan [{ $victim }] kepeken ilo pana
hud-chat-npc_explosion_kill_msg = { $attacker } li moli e jan [{ $victim }] kepeken seli pakala
hud-chat-npc_energy_kill_msg = { $attacker } li moli e jan [{ $victim }] kepeken nasa
hud-chat-npc_other_kill_msg = { $attacker } li moli e jan [{ $victim }]
hud-chat-fall_kill_msg = { "[" }{ $name }] li moli tan tawa anpa
hud-chat-suicide_msg = { "[" }{ $name }] li moli tan pakala sijelo tan ona
hud-chat-default_death_msg = { "[" }{ $name }] li moli
hud-chat-all = ale
hud-chat-chat_tab_hover_tooltip = o kepeken nena nanpa tu tawa nasin ante
hud-loot-pickup-msg-you =
    { $amount ->
        [1] sina kama jo e { $item }
       *[other] sina kama jo e { $item } { $amount }
    }
hud-loot-pickup-msg =
    { $amount ->
        [1] { $actor } li kama jo e { $item }
       *[other] { $actor } li kama jo e { $item } { $amount }
    }
hud-chat-singleplayer-motd1 = ma li tawa sina taso a! o open tawa...
hud-chat-singleplayer-motd2 = pilin pona li seme tawa sina?
hud-chat-connection_lost = toki tawa jan musi li pini. mi weka e ona lon tenpo { $time }.
hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $victim }] li moli tan ni: seli tan jan [{ $attacker }]
    .bleeding = { "[" }{ $victim }] li moli tan ni: pakala sijelo tan jan [{ $attacker }]
    .curse = { "[" }{ $victim }] li moli tan ni: ike nasa tan jan [{ $attacker }]
    .crippled = { "[" }{ $victim }] li moli tan ni: ike sijelo tan jan [{ $attacker }]
    .frozen = { "[" }{ $victim }] li moli tan ni: lete tan jan [{ $attacker }]
    .mysterious = { "[" }{ $victim }] li moli tan ni: nasa tan jan [{ $attacker }]
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] li moli tan ni: seli tan { $attacker }
    .bleeding = { "[" }{ $victim }] li moli tan ni: pakala sijelo tan jan { $attacker }
    .curse = { "[" }{ $victim }] li moli tan ni: ike nasa tan jan { $attacker }
    .crippled = { "[" }{ $victim }] li moli tan ni: ike sijelo tan jan { $attacker }
    .frozen = { "[" }{ $victim }] li moli tan ni: lete tan jan { $attacker }
    .mysterious = { "[" }{ $victim }] li moli tan ni: nasa tan jan { $attacker }
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] li moli tan ni: seli
    .bleeding = { "[" }{ $victim }] li moli tan ni: pakala sijelo
    .curse = { "[" }{ $victim }] li moli tan ni: ike nasa
    .crippled = { "[" }{ $victim }] li moli tan ni: ike sijelo
    .frozen = { "[" }{ $victim }] li moli tan ni: lete
    .mysterious = { "[" }{ $victim }] li moli tan ni: nasa
