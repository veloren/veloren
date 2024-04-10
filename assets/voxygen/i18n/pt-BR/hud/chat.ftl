## Eventos de Jogadores

hud-chat-online_msg = { "[" }{ $name }] está online.
hud-chat-offline_msg = { $name } ficou offline

## Avisos(buff)
## Mortes(buff)

hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $victim }] morreu de: queimadura causado por [{ $attacker }]
    .bleeding = { "[" }{ $victim }] morreu de: sangramento causado por [{ $attacker }]
    .curse = { "[" }{ $victim }] morreu de: maldição causado por [{ $attacker }]
    .crippled = { "[" }{ $victim }] morreu de: aleijamento causado por [{ $attacker }]
    .frozen = { "[" }{ $victim }] morreu de: congelamento causado por [{ $attacker }]
    .mysterious = { "[" }{ $victim }] morreu de: segredo causado por [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] morreu de: queimadura
    .bleeding = { "[" }{ $victim }] morreu de: sangramento
    .curse = { "[" }{ $victim }] morreu de: maldição
    .crippled = { "[" }{ $victim }] morreu de: aleijamento
    .frozen = { "[" }{ $victim }] morreu de: congelamento
    .mysterious = { "[" }{ $victim }] morreu de: segredo
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] morreu de: queimadura causado por { $attacker }
    .bleeding = { "[" }{ $victim }] morreu de: sangramento causado por { $attacker }
    .curse = { "[" }{ $victim }] morreu de: maldição causado por { $attacker }
    .crippled = { "[" }{ $victim }] morreu de: aleijamento causado por { $attacker }
    .frozen = { "[" }{ $victim }] morreu de: congelamento causado por { $attacker }
    .mysterious = { "[" }{ $victim }] morreu de: segredo causado por { $attacker }

## Mortes - PVP

hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }] derrotou [{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }] atirou em [{ $victim }]
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }] explodiu [{ $victim }]
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] matou [{ $victim }] com magia
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] matou [{ $victim }]

## Mortes - PVE

hud-chat-npc_melee_kill_msg = { $attacker } matou [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } atirou em [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } explodiu [{ $victim }]
hud-chat-npc_energy_kill_msg = { "[" }{ $attacker }] matou [{ $victim }] com magia
hud-chat-npc_other_kill_msg = { "[" }{ $attacker }] matou [{ $victim }]

## Outras mortes

hud-chat-fall_kill_msg = { "[" }{ $name }] morreu de dano de queda
hud-chat-suicide_msg = { "[" }{ $name }] morreu de dano autoinflingido
hud-chat-default_death_msg = { "[" }{ $name }] morreu

## Utilidades

hud-chat-all = Todos
hud-chat-chat_tab_hover_tooltip = Clique direito para configurar
hud-loot-pickup-msg =
    { $amount ->
        [1] { $actor } pegou { $item }
       *[other] { $actor } pegou { $amount }x { $item }
    }
hud-chat-goodbye = Até Logo!
hud-chat-connection_lost = Conexão perdida. Expulsando em { $time } segundos.
# Generic messages
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }
# Npc /tell messages, no gender info, sadly
hud-chat-tell-to-npc = Para [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-to = Para [{ $alias }]: { $msg }
# Npc /tell messages, no gender info, sadly
hud-chat-tell-from-npc = De [{ $alias }]: { $msg }
# Generic messages
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-from = Para [{ $alias }]: { $msg }
# Generic messages
hud-chat-message = { "[" }{ $alias }]: { $msg }
# HUD Pickup message
hud-loot-pickup-msg-you =
    { $amount ->
        [1] Você pegou { $item }
       *[other] Você pegou { $amount }x { $item }
    }
# Generic messages
hud-chat-message-with-name = { "[" }{ $alias }] { $name }: { $msg }
