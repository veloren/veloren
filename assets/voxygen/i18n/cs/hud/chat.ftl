hud-chat-all = Vše
hud-chat-chat_tab_hover_tooltip = Pravý klik pro nastavení
hud-chat-online_msg = { "[" }{ $name }] je teď online.
hud-chat-offline_msg = { "[" }{ $name }] se odhlásil.
hud-chat-default_death_msg = { "[" }{ $name }] zemřel/a
hud-chat-fall_kill_msg = { "[" }{ $name }] zemřel/a pádem
hud-chat-suicide_msg = { "[" }{ $name }] si způsobil/a zranění a zemřel/a
hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $victim }] uhořel/a způsobeno [{ $attacker }]
    .bleeding = { "[" }{ $victim }] vykrvácel/a způsobeno [{ $attacker }]
    .curse = { "[" }{ $victim }] zemřel/a kletbou způsobeno [{ $attacker }]
    .crippled = { "[" }{ $victim }] zemřel/a zmrzačením způsobeno [{ $attacker }]
    .frozen = { "[" }{ $victim }] umrzl/a způsobeno [{ $attacker }]
    .mysterious = { "[" }{ $victim }] záhadně zemřel s dopomocí [{ $attacker }]
hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }] porazil/a [{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }] zastřelil/a [{ $victim }]
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }] vyhodil/a do vzduchu [{ $victim }]
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] zabil/a [{ $victim }] kouzlem
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] uhořel/a
    .bleeding = { "[" }{ $victim }] vykrvácel/a
    .curse = { "[" }{ $victim }] zemřel/a kletbou
    .crippled = { "[" }{ $victim }] zemřel/a zmrzačením
    .frozen = { "[" }{ $victim }] umrzl/a
    .mysterious = { "[" }{ $victim }] záhadně zemřel/a
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] uhořel/a způsobeno { $attacker }
    .bleeding = { "[" }{ $victim }] vykrvácel/a způsobeno { $attacker }
    .curse = { "[" }{ $victim }] zemřel/a kletbou způsobeno { $attacker }
    .crippled = { "[" }{ $victim }] zemřel/a zmrzačením způsobeno { $attacker }
    .frozen = { "[" }{ $victim }] umrzl/a způsobeno { $attacker }
    .mysterious = { "[" }{ $victim }] záhadně zemřel s dopomocí { $attacker }
hud-chat-npc_melee_kill_msg = { $attacker } zabil/a [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } zastřelil/a [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } vyhodil/a do vzduchu [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } zabil/a [{ $victim }] kouzlem
hud-chat-npc_other_kill_msg = { $attacker } zabil/a [{ $victim }]
hud-chat-goodbye = Nashledanou!
hud-chat-connection_lost = Připojení ztraceno. Ukončuji za { $time } sekund.
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] zabil [{ $victim }]
hud-chat-tell-to = Pro [{ $alias }]: { $msg }
hud-chat-tell-from = Od [{ $alias }]: { $msg }
hud-chat-tell-to-npc = Pro [{ $alias }]: { $msg }
hud-chat-tell-from-npc = Od [{ $alias }]: { $msg }
hud-chat-message = { "[" }{ $alias }]: { $msg }
hud-chat-message-with-name = { "[" }{ $alias }] { $name }: { $msg }
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
hud-loot-pickup-msg-you =
    { $amount ->
        [1] Sebral jsi { $item }
       *[other] Sebral jsi { $amount }ks { $item }
    }
hud-loot-pickup-msg =
    { $amount ->
        [1] { $actor } sebral { $item }
       *[other] { $actor } sebral { $amount }ks { $item }
    }
