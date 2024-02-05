## Player events

hud-chat-online_msg = { "[" }{ $name }] est maintenant en ligne.
hud-chat-offline_msg = { "[" }{ $name }] s'est déconnecté.

## Buff deaths

hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $victim }] est mort de Mort: brûlé(e) causé par [{ $attacker }]
    .bleeding = { "[" }{ $victim }] est mort de Mort: saignement causé par [{ $attacker }]
    .curse = { "[" }{ $victim }] est mort de Mort: malédiction causé par [{ $attacker }]
    .crippled = { "[" }{ $victim }] est mort de Mort: estropié(e) causé par [{ $attacker }]
    .frozen = { "[" }{ $victim }] est mort de Mort: glacé(e) causé par [{ $attacker }]
    .mysterious = { "[" }{ $victim }] est mort de Mort: secrète causé par [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] est mort de Mort: brûlé(e)
    .bleeding = { "[" }{ $victim }] est mort de Mort: saignement
    .curse = { "[" }{ $victim }] est mort de Mort: malédiction
    .crippled = { "[" }{ $victim }] est mort de Mort: estropié(e)
    .frozen = { "[" }{ $victim }] est mort de Mort: glacé(e)
    .mysterious = { "[" }{ $victim }] est mort de Mort: secrète
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] est mort de Mort: brûlé(e) causé par { $attacker }
    .bleeding = { "[" }{ $victim }] est mort de Mort: saignement causé par { $attacker }
    .curse = { "[" }{ $victim }] est mort de Mort: malédiction causé par { $attacker }
    .crippled = { "[" }{ $victim }] est mort de Mort: estropié(e) causé par { $attacker }
    .frozen = { "[" }{ $victim }] est mort de Mort: glacé(e) causé par { $attacker }
    .mysterious = { "[" }{ $victim }] est mort de Mort: secrète causé par { $attacker }

## PvP deaths

hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }] a tué [{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }] a tiré sur [{ $victim }]
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }] a explosé [{ $victim }]
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] a tué [{ $victim }] avec de la magie
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] a tué [{ $victim }]

## PvE deaths

hud-chat-npc_melee_kill_msg = { $attacker } a tué [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } a tiré sur [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } a fait exploser [{ $victim }]
hud-chat-npc_energy_kill_msg = { "[" }{ $attacker }] a tué [{ $victim }] avec de la magie
hud-chat-npc_other_kill_msg = { "[" }{ $attacker }] a tué [{ $victim }]

## Other deaths

hud-chat-fall_kill_msg = { "[" }{ $name }] est mort de dégâts de chute
hud-chat-suicide_msg = { "[" }{ $name }] est mort des suites de ses propres blessures
hud-chat-default_death_msg = { "[" }{ $name }] est mort

## Utils

hud-chat-all = Global
hud-chat-chat_tab_hover_tooltip = Clique Droit pour ouvrir les paramètres
hud-loot-pickup-msg =
    { $actor } a récupéré { $amount ->
        [one] { $item }
       *[other] x{ $amount } { $item }s
    }
hud-chat-goodbye = Au revoir!
hud-chat-connection_lost = Connexion perdue. Expulsion dans { $time } secondes.
