## Player events
hud-chat-online_msg = [{ $name }] esta en linea
hud-chat-offline_msg = [{ $name }] se ha desconectado
## Buff outcomes
hud-outcome-burning = ha muerto por: quemadura
hud-outcome-curse = ha muerto por: maldición
hud-outcome-bleeding = ha muerto por: sangrado
hud-outcome-crippled = ha muerto por: lesión
hud-outcome-frozen = ha muerto por: congelamiento
hud-outcome-mysterious = ha muerto por: secreto
## Buff deaths
hud-chat-died_of_pvp_buff_msg = [{ $victim }] { $died_of_buff } causado por [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg = [{ $victim }] { $died_of_buff }
hud-chat-died_of_npc_buff_msg = [{ $victim }] { $died_of_buff } causado por { $attacker }
## PvP deaths
hud-chat-pvp_melee_kill_msg = [{ $attacker }] ha derrotado a [{ $victim }]
hud-chat-pvp_ranged_kill_msg = [{ $attacker }] disparó a [{ $victim }]
hud-chat-pvp_explosion_kill_msg = [{ $attacker }] hizo explotar a [{ $victim }]
hud-chat-pvp_energy_kill_msg = [{ $attacker }] mató a [{ $victim }] con magia
hud-chat-pvp_other_kill_msg = [{ $attacker }] mató a [{ $victim }]
## PvE deaths
hud-chat-npc_melee_kill_msg = { $attacker } ha matado a [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } disparó a [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } hizo explotar a [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } mató a [{ $victim }] con magia
hud-chat-npc_other_kill_msg = { $attacker } mató a [{ $victim }]
## Other deaths
hud-chat-environmental_kill_msg = [{ $name }] murió en { $environment }
hud-chat-fall_kill_msg = [{ $name }] murio por daño de caida
hud-chat-suicide_msg = [{ $name }] murió por heridas autoinfligidas
hud-chat-default_death_msg = [{ $name }] murió
## Utils
hud-chat-all = Todos
hud-chat-you = Tú
hud-chat-mod = Moderador
hud-chat-chat_tab_hover_tooltip = Click derecho para opciones
hud-loot-pickup-msg = {$actor} Recogio { $amount ->
   [one] { $item }
   *[other] {$amount}x {$item}
}
hud-chat-loot_fail = ¡Tu inventario está lleno!
hud-chat-goodbye = ¡Adiós!
hud-chat-connection_lost = Conexión perdida. Desconectando en { $time } segundos.