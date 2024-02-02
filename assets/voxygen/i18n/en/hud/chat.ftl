## Player events

hud-chat-online_msg = [{ $name }] is online now
hud-chat-offline_msg = [{ $name }] went offline
hud-chat-goodbye = Goodbye!
hud-chat-connection_lost = Connection lost. Kicking in { $time } seconds.

## Buff deaths

hud-chat-died_of_pvp_buff_msg =
 .burning = [{ $victim }] died of: burning caused by [{ $attacker }]
 .bleeding = [{ $victim }] died of: bleeding caused by [{ $attacker }]
 .curse = [{ $victim }] died of: curse caused by [{ $attacker }]
 .crippled = [{ $victim }] died of: crippled caused by [{ $attacker }]
 .frozen = [{ $victim }] died of: frozen caused by [{ $attacker }]
 .mysterious = [{ $victim }] died of: secret caused by [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg =
 .burning = [{ $victim }] died of: burning
 .bleeding = [{ $victim }] died of: bleeding
 .curse = [{ $victim }] died of: curse
 .crippled = [{ $victim }] died of: crippled
 .frozen = [{ $victim }] died of: frozen
 .mysterious = [{ $victim }] died of: secret
hud-chat-died_of_npc_buff_msg =
 .burning = [{ $victim }] died of: burning caused by { $attacker }
 .bleeding = [{ $victim }] died of: bleeding caused by { $attacker }
 .curse = [{ $victim }] died of: curse caused by { $attacker }
 .crippled = [{ $victim }] died of: crippled caused by { $attacker }
 .frozen = [{ $victim }] died of: frozen caused by { $attacker }
 .mysterious = [{ $victim }] died of: secret caused by { $attacker }

## Other PvP deaths

hud-chat-pvp_melee_kill_msg = [{ $attacker }] defeated [{ $victim }]
hud-chat-pvp_ranged_kill_msg = [{ $attacker }] shot [{ $victim }]
hud-chat-pvp_explosion_kill_msg = [{ $attacker }] blew up [{ $victim }]
hud-chat-pvp_energy_kill_msg = [{ $attacker }] killed [{ $victim }] with magic
hud-chat-pvp_other_kill_msg = [{ $attacker }] killed [{ $victim }]

## PvE deaths

hud-chat-npc_melee_kill_msg = { $attacker } killed [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } shot [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } blew up [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } killed [{ $victim }] with magic
hud-chat-npc_other_kill_msg = { $attacker } killed [{ $victim }]

## Other deaths

hud-chat-fall_kill_msg = [{ $name }] died from fall damage
hud-chat-suicide_msg = [{ $name }] died from self-inflicted wounds
hud-chat-default_death_msg = [{ $name }] died

## Result of our i18n antipatterns

hud-chat-you = You

## Random stuff

hud-chat-all = All
hud-chat-chat_tab_hover_tooltip = Right click for settings

## HUD Pickup message

hud-loot-pickup-msg = { $amount ->
   [one] { $actor } picked up { $item }
   *[other] { $actor } picked up {$amount}x {$item}
}
