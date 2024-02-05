## Utils

hud-chat-all = Alle
hud-chat-chat_tab_hover_tooltip = Rechtsklick für Einstellungen
hud-loot-pickup-msg =
    { $actor } nahm { $amount ->
        [one] { $item }
       *[other] { $amount }x { $item }
    } auf

## Player events

hud-chat-online_msg = { "[" }{ $name }] ist nun online
hud-chat-offline_msg = { "[" }{ $name }] ging offline

## Other deaths

hud-chat-default_death_msg = { "[" }{ $name }] starb
hud-chat-fall_kill_msg = { "[" }{ $name }] starb durch Fallschaden
hud-chat-suicide_msg = { "[" }{ $name }] beging Selbstmord

## Buff, PvE, PvP deaths

hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $victim }] starb an An Verbrennung gestorben von [{ $attacker }]
    .bleeding = { "[" }{ $victim }] starb an Verblutet von [{ $attacker }]
    .curse = { "[" }{ $victim }] starb an An Verfluchung gestorben von [{ $attacker }]
    .crippled = { "[" }{ $victim }] starb an An Verkrüpplung gestorben von [{ $attacker }]
    .frozen = { "[" }{ $victim }] starb an Erfroren von [{ $attacker }]
    .mysterious = { "[" }{ $victim }] starb an Unter geheimnisvollen Umständen gestorben von [{ $attacker }]
hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }] vernichtete [{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }] erschoss [{ $victim }]
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }] sprengte [{ $victim }] aus dem Leben
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] tötete [{ $victim }] mit Magie
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] tötete [{ $victim }]
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] starb an An Verbrennung gestorben
    .bleeding = { "[" }{ $victim }] starb an Verblutet
    .curse = { "[" }{ $victim }] starb an An Verfluchung gestorben
    .crippled = { "[" }{ $victim }] starb an An Verkrüpplung gestorben
    .frozen = { "[" }{ $victim }] starb an Erfroren
    .mysterious = { "[" }{ $victim }] starb an Unter geheimnisvollen Umständen gestorben
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] starb an An Verbrennung gestorben von { $attacker }
    .bleeding = { "[" }{ $victim }] starb an Verblutet von { $attacker }
    .curse = { "[" }{ $victim }] starb an An Verfluchung gestorben von { $attacker }
    .crippled = { "[" }{ $victim }] starb an An Verkrüpplung gestorben von { $attacker }
    .frozen = { "[" }{ $victim }] starb an Erfroren von { $attacker }
    .mysterious = { "[" }{ $victim }] starb an Unter geheimnisvollen Umständen gestorben von { $attacker }
hud-chat-npc_melee_kill_msg = { $attacker } tötete [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } erschoss [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } sprengte [{ $victim }] aus dem Leben
hud-chat-npc_energy_kill_msg = { $attacker } tötete [{ $victim }] mit Magie
hud-chat-npc_other_kill_msg = { $attacker } tötete [{ $victim }]
hud-chat-goodbye = Auf Wiedersehen!
hud-chat-connection_lost = Verbindungsabbruch. Du wirst in { $time } Sekunden gekickt.
