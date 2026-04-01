## General

hud-chat-online_msg = { $name } se ha conectado.
hud-chat-offline_msg = { $name } se ha desconectado.
hud-chat-connection_lost = Conexión perdida. Saliendo en { $time } segundos.
hud-chat-goodbye = ¡Adiós!
hud-chat-chat_tab_hover_tooltip = Click derecho para opciones
hud-chat-all = Global

## Maneras de morirse

hud-chat-default_death_msg = { $name } ha muerto
hud-chat-suicide_msg = { $name } se ha suicidado
hud-chat-fall_kill_msg = { $name } ha muerto por caer desde demasiada altura
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { $victim } murió de: quemado
    .bleeding = { $victim } murió de: desangrado
    .curse = { $victim } murió de: una maldición
    .crippled = { $victim } murió de: heridas graves
    .frozen = { $victim } murió de: congelado
    .mysterious = { $victim } murió de: manera misteriosa

## PvE

hud-chat-npc_other_kill_msg = { $attacker } ha matado a { $victim }
hud-chat-npc_melee_kill_msg = { $attacker } ha matado a { $victim } con un arma cuerpo a cuerpo
hud-chat-npc_ranged_kill_msg = { $attacker } ha matado a { $victim } con un arma de proyectil
hud-chat-npc_explosion_kill_msg = { $attacker } ha hecho explotar a { $victim }
hud-chat-npc_energy_kill_msg = { $attacker } ha matado a { $victim } con magia
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] murió de: quemado causado por { $attacker }
    .bleeding = { "[" }{ $victim }] murió de: desangrado causado por { $attacker }
    .curse = { "[" }{ $victim }] murió de: una maldición causada por { $attacker }
    .crippled = { "[" }{ $victim }] murió de: heridas graves causadas por { $attacker }
    .frozen = { "[" }{ $victim }] murió de: congelado causado por { $attacker }
    .mysterious = { "[" }{ $victim }] murió de: manera misteriosa causado por { $attacker }

## PvP

hud-chat-pvp_other_kill_msg = { $attacker } ha matado a { $victim }
hud-chat-pvp_melee_kill_msg = { $attacker } ha matado a { $victim } con un arma cuerpo a cuerpo
hud-chat-pvp_ranged_kill_msg = { $attacker } ha matado a { $victim } con un arma de proyectil
hud-chat-pvp_explosion_kill_msg = { $attacker } ha hecho explotar a { $victim }
hud-chat-pvp_energy_kill_msg = { $attacker } ha matado a { $victim } con magia
hud-chat-died_of_pvp_buff_msg =
    .burning = { $victim } murió de: quemado causado por { $attacker }
    .bleeding = { $victim } murió de: desangrado causado por { $attacker }
    .curse = { $victim } murió de: una maldición causada por { $attacker }
    .crippled = { $victim } murió de: heridas graves causadas por { $attacker }
    .frozen = { $victim } murió de: congelado causado por { $attacker }
    .mysterious = { $victim } murió de: manera misteriosa causado por { $attacker }

## Inventario

hud-loot-pickup-msg =
    { $amount ->
        [1] { $actor } picked up { $item }
       *[other] { $actor } picked up { $amount }x { $item }
    }
# Player /tell messages, $user_gender should be available
hud-chat-tell-from = De [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-to-npc = Para [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-to = Para [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message = { "[" }{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message-with-name = { "[" }{ $alias }] { $name }: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-from-npc = De [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }
# HUD Pickup message
hud-loot-pickup-msg-you =
    { $amount ->
        [1] { $item }
       *[other] { $amount }x { $item }
    }
hud-chat-singleplayer-motd1 = ¡Todo un mundo para ti! Hora de estirar...
hud-chat-singleplayer-motd2 = ¿Qué tal la serenidad?
