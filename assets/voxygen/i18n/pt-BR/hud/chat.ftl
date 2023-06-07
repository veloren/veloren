## Eventos de Jogadores
hud-chat-online_msg = [{ $name }] está online.
hud-chat-offline_msg = { $name } está offline
## Avisos(buff)
hud-outcome-burning = morreu de: queimadura
hud-outcome-curse = morreu de: maldição
hud-outcome-bleeding = morreu de: sangramento
hud-outcome-crippled = morreu de: aleijamento
hud-outcome-frozen = morreu de: congelamento
hud-outcome-mysterious = morreu de: segredo
## Mortes(buff)
hud-chat-died_of_pvp_buff_msg = [{ $victim }] { $died_of_buff } causado por [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg = [{ $victim }] { $died_of_buff }
hud-chat-died_of_npc_buff_msg = [{ $victim }] { $died_of_buff } causado por { $attacker }
## Mortes - PVP
hud-chat-pvp_melee_kill_msg = [{ $attacker }] derrotou [{ $victim }]
hud-chat-pvp_ranged_kill_msg = [{ $attacker }] atirou em [{ $victim }]
hud-chat-pvp_explosion_kill_msg = [{ $attacker }] explodiu [{ $victim }]
hud-chat-pvp_energy_kill_msg = [{ $attacker }] matou [{ $victim }] com magia
hud-chat-pvp_other_kill_msg = [{ $attacker }] matou [{ $victim }]
## Mortes - PVE
hud-chat-npc_melee_kill_msg = { $attacker } matou [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } atirou em [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } explodiu [{ $victim }]
hud-chat-npc_energy_kill_msg = [{ $attacker }] matou [{ $victim }] com magia
hud-chat-npc_other_kill_msg = [{ $attacker }] matou [{ $victim }]
## Outras mortes
hud-chat-environmental_kill_msg = [{ $name }] morreu de { $environment }
hud-chat-fall_kill_msg = [{ $name }] morreu de dano de queda
hud-chat-suicide_msg = [{ $name }] morreu de dano autoinflingido
hud-chat-default_death_msg = [{ $name }] morreu
## Utilidades
hud-chat-all = Todos
hud-chat-you = Você
hud-chat-chat_tab_hover_tooltip = Clique direito para configurar
hud-loot-pickup-msg = { $actor -> 
      [You] Você pegou
      *[other] { $actor } pegou
} { $amount ->
   [one] { $item }
   *[other] {$amount}x {$item}
}
hud-chat-loot_fail = Seu Inventário está cheio!
hud-chat-goodbye = Até Logo!
hud-chat-connection_lost = Conexão perdida. Expulsando em { $time } segundos.
