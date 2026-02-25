## Player events

hud-chat-online_msg = { "[" }{ $name }] är online nu.
hud-chat-offline_msg = { "[" }{ $name }] är inte längre online.

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
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] dödade [{ $victim }] med magi
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] dödade [{ $victim }]

## PvE deaths

hud-chat-npc_melee_kill_msg = { $attacker } dödade [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } sköt [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } sprängde [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } dödade [{ $victim }] med magi
hud-chat-npc_other_kill_msg = { $attacker } dödade [{ $victim }]

## Other deaths

hud-chat-fall_kill_msg = { "[" }{ $name }] föll till sin död
hud-chat-suicide_msg = { "[" }{ $name }] dog av självförvållade skador
hud-chat-default_death_msg = { "[" }{ $name }] dog

## Utils

hud-chat-all = Alla
hud-chat-chat_tab_hover_tooltip = Högerklicka för inställningar
hud-loot-pickup-msg =
    { $amount ->
        [1] { $actor } plockade upp { $item }
       *[other] { $actor } plockade upp { $amount }x { $item }
    }
hud-chat-goodbye = Hejdå!
hud-chat-connection_lost = Anslutningen bröts. Sparkas ut om { $time } sekunder.
# Player /tell messages, $user_gender should be available
hud-chat-tell-to = Till [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-from = Från [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-from-npc = Från [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-to-npc = Till [{ $alias }]: { $msg }
# HUD Pickup message
hud-loot-pickup-msg-you =
    { $amount ->
        [1] Du plockade upp { $item }
       *[other] Du plockade upp { $amount }x { $item }
    }
# Player /tell messages, $user_gender should be available
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message = { "[" }{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message-with-name = { "[" }{ $alias }] { $name }: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
hud-chat-singleplayer-motd1 = En hel värld till dig själv! Dags att stretcha...
hud-chat-singleplayer-motd2 = hur är sinnesron?
