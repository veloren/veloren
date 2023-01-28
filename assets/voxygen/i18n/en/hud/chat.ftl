## Player events
hud-chat-online_msg = [{ $name }] is online now
hud-chat-offline_msg = [{ $name }] went offline
## Buff outcomes
hud-outcome-burning = died of: burning
hud-outcome-curse = died of: curse
hud-outcome-bleeding = died of: bleeding
hud-outcome-crippled = died of: crippled
hud-outcome-frozen = died of: frozen
hud-outcome-mysterious = died of: secret
## Buff deaths
hud-chat-died_of_pvp_buff_msg = [{ $victim }] { $died_of_buff } caused by [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg = [{ $victim }] { $died_of_buff }
hud-chat-died_of_npc_buff_msg = [{ $victim }] { $died_of_buff } caused by { $attacker }
## PvP deaths
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
hud-chat-environmental_kill_msg = [{ $name }] died in { $environment }
hud-chat-fall_kill_msg = [{ $name }] died from fall damage
hud-chat-suicide_msg = [{ $name }] died from self-inflicted wounds
hud-chat-default_death_msg = [{ $name }] died
## Utils
hud-chat-all = All
hud-chat-you = You
hud-chat-chat_tab_hover_tooltip = Right click for settings
hud-loot-pickup-msg = {$actor} picked up { $amount ->
   [one] { $item }
   *[other] {$amount}x {$item}
}
hud-chat-loot_fail = Your Inventory is full!
hud-chat-goodbye = Goodbye!
hud-chat-connection_lost = Connection lost. Kicking in { $time } seconds.