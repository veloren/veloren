## General
hud-chat-online_msg = { $name } se ha conectado.
hud-chat-offline_msg = { $name } se ha desconectado.
hud-chat-connection_lost = Conexión perdida. Saliendo en { $time } segundos.
hud-chat-goodbye = ¡Adiós!
hud-chat-chat_tab_hover_tooltip = Click derecho para opciones
hud-chat-all = Global
hud-chat-you = Tú
hud-chat-mod = Mod

## Maneras de morirse
hud-chat-default_death_msg = { $name } ha muerto
hud-chat-suicide_msg = { $name } se ha suicidado
hud-outcome-burning = se ha quemado hasta morir
hud-outcome-curse = ha sido víctima de una maldición
hud-outcome-bleeding = ha muerto desangrado
hud-outcome-crippled = ha muerto por heridas graves
hud-outcome-frozen = ha muerto de hipotermia
hud-outcome-mysterious = ha muerto de manera misteriosa
hud-chat-fall_kill_msg = { $name } ha muerto por caer desde demasiada altura
hud-chat-environmental_kill_msg = { $name } ha muerto en { $environment }
hud-chat-died_of_buff_nonexistent_msg = { $victim } { $died_of_buff }

## PvE
hud-chat-npc_other_kill_msg = { $attacker } ha matado a { $victim }
hud-chat-npc_melee_kill_msg = { $attacker } ha matado a { $victim } con un arma cuerpo a cuerpo
hud-chat-npc_ranged_kill_msg = { $attacker } ha matado a { $victim } con un arma de proyectil
hud-chat-npc_explosion_kill_msg = { $attacker } ha hecho explotar a { $victim }
hud-chat-npc_energy_kill_msg = { $attacker } ha matado a { $victim } con magia
hud-chat-died_of_npc_buff_msg = { $victim } { $died_of_buff } a manos de { $attacker }

## PvP
hud-chat-pvp_other_kill_msg = { $attacker } ha matado a { $victim }
hud-chat-pvp_melee_kill_msg = { $attacker } ha matado a { $victim } con un arma cuerpo a cuerpo
hud-chat-pvp_ranged_kill_msg = { $attacker } ha matado a { $victim } con un arma de proyectil
hud-chat-pvp_explosion_kill_msg = { $attacker } ha hecho explotar a { $victim }
hud-chat-pvp_energy_kill_msg = { $attacker } ha matado a { $victim } con magia
hud-chat-died_of_pvp_buff_msg = { $victim } { $died_of_buff } a manos de { $attacker }

## Inventario
hud-chat-loot_fail = ¡Tu inventario está lleno!
hud-loot-pickup-msg =
   { $actor ->
      [You] {"\u0000"}
      *[other] { $actor } ha obtenido 
   } { $amount ->
      [one] { $item }
      *[other] { $amount }x { $item }
   }