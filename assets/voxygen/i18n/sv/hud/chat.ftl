## Player events

hud-chat-online_msg = { "[" }{ $name }] är inloggad nu
hud-chat-offline_msg = { "[" }{ $name }] loggade ut

## Buff deaths

hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $victim }] dödsorsak: eld orsakad av [{ $attacker }]
    .bleeding = { "[" }{ $victim }] dödsorsak: blodförlust orsakad av [{ $attacker }]
    .curse = { "[" }{ $victim }] dödsorsak: trolldom orsakad av [{ $attacker }]
    .crippled = { "[" }{ $victim }] dödsorsak: allvarliga skador orsakad av [{ $attacker }]
    .frozen = { "[" }{ $victim }] dödsorsak: förfrysning orsakad av [{ $attacker }]
    .mysterious = { "[" }{ $victim }] dödsorsak: hemlig orsakad av [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] dödsorsak: eld
    .bleeding = { "[" }{ $victim }] dödsorsak: blodförlust
    .curse = { "[" }{ $victim }] dödsorsak: trolldom
    .crippled = { "[" }{ $victim }] dödsorsak: allvarliga skador
    .frozen = { "[" }{ $victim }] dödsorsak: förfrysning
    .mysterious = { "[" }{ $victim }] dödsorsak: hemlig
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] dödsorsak: eld orsakad av { $attacker }
    .bleeding = { "[" }{ $victim }] dödsorsak: blodförlust orsakad av { $attacker }
    .curse = { "[" }{ $victim }] dödsorsak: trolldom orsakad av { $attacker }
    .crippled = { "[" }{ $victim }] dödsorsak: allvarliga skador orsakad av { $attacker }
    .frozen = { "[" }{ $victim }] dödsorsak: förfrysning orsakad av { $attacker }
    .mysterious = { "[" }{ $victim }] dödsorsak: hemlig orsakad av { $attacker }

## PvP deaths

hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }] besegrade [{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }] sköt [{ $victim }]
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }] sprängde [{ $victim }]
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] dödade [{ $victim }] med trolldom
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] dödade [{ $victim }]

## PvE deaths

hud-chat-npc_melee_kill_msg = { $attacker } dödade [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } sköt [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } sprängde [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } dödade [{ $victim }] med trolldom
hud-chat-npc_other_kill_msg = { $attacker } dödade [{ $victim }]

## Other deaths

hud-chat-fall_kill_msg = { "[" }{ $name }] föll till sin död
hud-chat-suicide_msg = { "[" }{ $name }] dog av självförvållade skador
hud-chat-default_death_msg = { "[" }{ $name }] dog

## Utils

hud-chat-all = Alla
hud-chat-chat_tab_hover_tooltip = Högerklicka för inställningar
hud-loot-pickup-msg =
    { $actor } plockade upp { $amount ->
        [one] { $item }
       *[other] { $amount }x { $item }
    }
hud-chat-goodbye = Hejdå!
hud-chat-connection_lost = Anslutningen bröts. Sparkas ut om { $time } sekunder.
