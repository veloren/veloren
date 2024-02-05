## Player events

hud-chat-online_msg = { "[" }{ $name }] esta en linea
hud-chat-offline_msg = { "[" }{ $name }] se ha desconectado

## Buff deaths

hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $victim }] ha muerto por: quemadura causado por [{ $attacker }]
    .bleeding = { "[" }{ $victim }] ha muerto por: sangrado causado por [{ $attacker }]
    .curse = { "[" }{ $victim }] ha muerto por: maldición causado por [{ $attacker }]
    .crippled = { "[" }{ $victim }] ha muerto por: lesión causado por [{ $attacker }]
    .frozen = { "[" }{ $victim }] ha muerto por: congelamiento causado por [{ $attacker }]
    .mysterious = { "[" }{ $victim }] ha muerto por: secreto causado por [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] ha muerto por: quemadura
    .bleeding = { "[" }{ $victim }] ha muerto por: sangrado
    .curse = { "[" }{ $victim }] ha muerto por: maldición
    .crippled = { "[" }{ $victim }] ha muerto por: lesión
    .frozen = { "[" }{ $victim }] ha muerto por: congelamiento
    .mysterious = { "[" }{ $victim }] ha muerto por: secreto
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] ha muerto por: quemadura causado por { $attacker }
    .bleeding = { "[" }{ $victim }] ha muerto por: sangrado causado por { $attacker }
    .curse = { "[" }{ $victim }] ha muerto por: maldición causado por { $attacker }
    .crippled = { "[" }{ $victim }] ha muerto por: lesión causado por { $attacker }
    .frozen = { "[" }{ $victim }] ha muerto por: congelamiento causado por { $attacker }
    .mysterious = { "[" }{ $victim }] ha muerto por: secreto causado por { $attacker }

## PvP deaths

hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }] ha derrotado a [{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }] disparó a [{ $victim }]
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }] hizo explotar a [{ $victim }]
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] mató a [{ $victim }] con magia
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] mató a [{ $victim }]

## PvE deaths

hud-chat-npc_melee_kill_msg = { $attacker } ha matado a [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } disparó a [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } hizo explotar a [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } mató a [{ $victim }] con magia
hud-chat-npc_other_kill_msg = { $attacker } mató a [{ $victim }]

## Other deaths

hud-chat-fall_kill_msg = { "[" }{ $name }] murio por daño de caida
hud-chat-suicide_msg = { "[" }{ $name }] murió por heridas autoinfligidas
hud-chat-default_death_msg = { "[" }{ $name }] murió

## Utils

hud-chat-all = Todos
hud-chat-chat_tab_hover_tooltip = Click derecho para opciones
hud-loot-pickup-msg =
    { $actor } Recogio { $amount ->
        [one] { $item }
       *[other] { $amount }x { $item }
    }
hud-chat-goodbye = ¡Adiós!
hud-chat-connection_lost = Conexión perdida. Desconectando en { $time } segundos.
