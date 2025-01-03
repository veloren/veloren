## Player events

hud-chat-online_msg = { "[" }{ $name }] est maintenant en ligne.
hud-chat-offline_msg = { "[" }{ $name }] s'est déconnecté.

## Buff deaths

hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $victim }] est mort brûlé(e) par [{ $attacker }]
    .bleeding = { "[" }{ $victim }] est mort de saignement causé par [{ $attacker }]
    .curse = { "[" }{ $victim }] est mort d'une malédiction causé par [{ $attacker }]
    .crippled = { "[" }{ $victim }] est mort estropié(e) par [{ $attacker }]
    .frozen = { "[" }{ $victim }] est mort glacé(e) par [{ $attacker }]
    .mysterious = { "[" }{ $victim }] est mort de cause mystérieuse par [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] est mort brûlé(e)
    .bleeding = { "[" }{ $victim }] est mort de saignement
    .curse = { "[" }{ $victim }] est mort de malédiction
    .crippled = { "[" }{ $victim }] est mort estropié(e)
    .frozen = { "[" }{ $victim }] est mort glacé(e)
    .mysterious = { "[" }{ $victim }] est mort de cause mystérieuse
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] est mort brûlé(e) par { $attacker }
    .bleeding = { "[" }{ $victim }] est mort de saignement causé par { $attacker }
    .curse = { "[" }{ $victim }] est mort d'une malédiction causé par { $attacker }
    .crippled = { "[" }{ $victim }] est mort estropié(e) par { $attacker }
    .frozen = { "[" }{ $victim }] est mort glacé(e) par { $attacker }
    .mysterious = { "[" }{ $victim }] est mort de cause mystérieuse par { $attacker }

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
    { $amount ->
        [1] { $actor } a ramassé { $item }
       *[other] { $actor } a ramassé { $amount }x { $item }
    }
hud-chat-goodbye = Au revoir!
hud-chat-connection_lost = Connexion perdue. Expulsion dans { $time } secondes.
# Player /tell messages, $user_gender should be available
hud-chat-tell-to = Pour [{ $alias }] : { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-from = De [{ $alias }] : { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-to-npc = À [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-from-npc = De [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message = { "[" }{ $alias }] : { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message-with-name = { "[" }{ $alias }] { $name } : { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message-in-group = ({ $group }) [{ $alias }] : { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name } : { $msg }
# HUD Pickup message
hud-loot-pickup-msg-you =
    { $amount ->
        [1] Vous avez récupéré { $item }
       *[autre] vous avez récupéré { $amount }x { $item }
    }
