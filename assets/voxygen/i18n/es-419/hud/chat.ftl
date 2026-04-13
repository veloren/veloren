## Player events

hud-chat-online_msg = { "[" }{ $name }] está en linea.
hud-chat-offline_msg = { "[" }{ $name }] se ha desconectado.

## Buff deaths

hud-chat-died_of_pvp_buff_msg =
    .burning = { $victim } murió de: quemado causado por { $attacker }
    .bleeding = { $victim } murió de: desangrado causado por { $attacker }
    .curse = { $victim } murió de: una maldición causada por { $attacker }
    .crippled = { $victim } murió de: heridas graves causadas por { $attacker }
    .frozen = { $victim } murió de: congelado causado por { $attacker }
    .mysterious = { $victim } murió de: manera misteriosa causado por { $attacker }
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { $victim } murió de: quemado
    .bleeding = { $victim } murió de: desangrado
    .curse = { $victim } murió de: una maldición
    .crippled = { $victim } murió de: heridas graves
    .frozen = { $victim } murió de: congelado
    .mysterious = { $victim } murió de: manera misteriosa
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] murió de: quemado causado por { $attacker }
    .bleeding = { "[" }{ $victim }] murió de: desangrado causado por { $attacker }
    .curse = { "[" }{ $victim }] murió de: una maldición causada por { $attacker }
    .crippled = { "[" }{ $victim }] murió de: heridas graves causadas por { $attacker }
    .frozen = { "[" }{ $victim }] murió de: congelado causado por { $attacker }
    .mysterious = { "[" }{ $victim }] murió de: manera misteriosa causado por { $attacker }

## PvP deaths

hud-chat-pvp_melee_kill_msg = { $attacker } ha matado a { $victim } con un arma cuerpo a cuerpo
hud-chat-pvp_ranged_kill_msg = { $attacker } ha matado a { $victim } con un arma de proyectil
hud-chat-pvp_explosion_kill_msg = { $attacker } ha hecho explotar a { $victim }
hud-chat-pvp_energy_kill_msg = { $attacker } ha matado a { $victim } con magia
hud-chat-pvp_other_kill_msg = { $attacker } ha matado a { $victim }

## PvE deaths

hud-chat-npc_melee_kill_msg = { $attacker } ha matado a { $victim } con un arma cuerpo a cuerpo
hud-chat-npc_ranged_kill_msg = { $attacker } ha matado a { $victim } con un arma de proyectil
hud-chat-npc_explosion_kill_msg = { $attacker } ha hecho explotar a { $victim }
hud-chat-npc_energy_kill_msg = { $attacker } ha matado a { $victim } con magia
hud-chat-npc_other_kill_msg = { $attacker } ha matado a { $victim }

## Other deaths

hud-chat-fall_kill_msg = { $name } ha muerto por caer desde demasiada altura
hud-chat-suicide_msg = { $name } se ha suicidado
hud-chat-default_death_msg = { $name } ha muerto

## Utils

hud-chat-all = Global
hud-chat-chat_tab_hover_tooltip = Click derecho para opciones
hud-loot-pickup-msg =
    { $amount ->
        [1] { $actor } picked up { $item }
       *[other] { $actor } picked up { $amount }x { $item }
    }
hud-chat-goodbye = ¡Adiós!
hud-chat-connection_lost = Conexión perdida. Saliendo en { $time } segundos.
hud-chat-tell-to = Para [{ $alias }]: { $msg }
hud-chat-tell-from = De [{ $alias }]: { $msg }
hud-chat-tell-to-npc = Para [{ $alias }]: { $msg }
hud-chat-tell-from-npc = De [{ $alias }]: { $msg }
hud-chat-message = { "[" }{ $alias }]: { $msg }
hud-chat-message-with-name = { "[" }{ $alias }] { $name }: { $msg }
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }
hud-loot-pickup-msg-you =
    { $amount ->
        [1] { $item }
       *[other] { $amount }x { $item }
    }
hud-chat-singleplayer-motd1 = ¡Todo un mundo para ti! Hora de estirar...
hud-chat-singleplayer-motd2 = ¿Qué tal la serenidad?
