hud-chat-all = Усе
hud-chat-chat_tab_hover_tooltip = ПКМ для наладаў
hud-chat-online_msg = { "[" }{ $name }] зайшоў у сетку.
hud-chat-offline_msg = { "[" }{ $name }] больш не ў сетцы.
hud-chat-default_death_msg = { "[" }{ $name }] памёр(-ла)
hud-chat-fall_kill_msg = { "[" }{ $name }] разбіўся(-лася) насмерць
hud-chat-suicide_msg = { "[" }{ $name }] здзейсніў(-ла) самагубства
hud-chat-died_of_pvp_buff_msg =
    .burning =
        { $attacker_gender ->
            [she] { "[" }{ $attacker }] спаліла [{ $victim }] жыўцом
           *[he] { "[" }{ $attacker }] спаліў [{ $victim }] жыўцом
        }
    .bleeding =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад крывацёку, выклікаага [{ $attacker }]
           *[he] { "[" }{ $victim }] памёр ад крывацёку, выклікаага [{ $attacker }]
        }
    .curse =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад праклёну, накладзенага [{ $attacker }]
           *[he] { "[" }{ $victim }] памёр ад праклёну, накладзенага [{ $attacker }]
        }
    .crippled =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад траўм, нанесёных [{ $attacker }]
           *[he] { "[" }{ $victim }] памёр ад траўм, нанесёных [{ $attacker }]
        }
    .frozen =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад холаду, з-за [{ $attacker }]
           *[he] { "[" }{ $victim }] памёр ад холаду, з-за [{ $attacker }]
        }
    .mysterious =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ... з-за [{ $attacker }] ... як?
           *[he] { "[" }{ $victim }] памёр ... з-за [{ $attacker }] ... як?
        }
hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }] перамог(-ла) [{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }] застрэліў(-ла) [{ $victim }]
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }] падарваў(-ла) [{ $victim }]
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] забіў(-ла) [{ $victim }] чарамі
hud-chat-died_of_buff_nonexistent_msg =
    .burning =
        { $attacker_gender ->
            [she] { "[" }{ $victim }] згарэла жыўцом
           *[he] { "[" }{ $victim }] згарэў жыўцом
        }
    .bleeding =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад крывацёку
           *[he] { "[" }{ $victim }] памёр ад крывацёку
        }
    .curse =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад праклёну
           *[he] { "[" }{ $victim }] памёр ад праклёну
        }
    .crippled =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад траўм
           *[he] { "[" }{ $victim }] памёр ад траўм
        }
    .frozen =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад холаду
           *[he] { "[" }{ $victim }] памёр ад холаду
        }
    .mysterious =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ... як?
           *[he] { "[" }{ $victim }] памёр ... як?
        }
hud-chat-died_of_npc_buff_msg =
    .burning =
        { $attacker_gender ->
            [she] { "[" }{ $attacker }] спаліла { $victim }]жыўцом
           *[he] { "[" }{ $attacker }] спаліў { $victim } жыўцом
        }
    .bleeding =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад крывацёку, выклікаага { $attacker }
           *[he] { "[" }{ $victim }] памёр ад крывацёку, выклікаага { $attacker }
        }
    .curse =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад праклёну, накладзенага { $attacker }
           *[he] { "[" }{ $victim }] памёр ад праклёну, накладзенага { $attacker }
        }
    .crippled =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад траўм, нанесёных { $attacker }
           *[he] { "[" }{ $victim }] памёр ад траўм, нанесёных { $attacker }
        }
    .frozen =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ад холаду, з-за { $attacker }
           *[he] { "[" }{ $victim }] памёр ад холаду, з-за { $attacker }
        }
    .mysterious =
        { $victim_gender ->
            [she] { "[" }{ $victim }] памерла ... з-за { $attacker } ... як?
           *[he] { "[" }{ $victim }] памёр ... з-за { $attacker } ... як?
        }
hud-chat-npc_melee_kill_msg = { $attacker } забіў(-ла) [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } застрэліў(-ла) [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } падарваў(-ла) [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } забіў(-ла) [{ $victim }] чарамі
hud-chat-npc_other_kill_msg = { $attacker } забіў(-ла) [{ $victim }]
hud-chat-goodbye = Да пабачэння!
hud-chat-connection_lost = Злучэнне згублена. Вас выштурхнуць праз { $time } сек.
# Generic messages
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-to = Да [{ $alias }]: { $msg }
# Player /tell messages, $user_gender should be available
hud-chat-tell-from = Ад [{ $alias }]: { $msg }
# Other PvP deaths, both $attacker_gender and $victim_gender are available
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] забіў [{ $victim }]
# HUD Pickup message
hud-loot-pickup-msg-you =
    { $amount ->
        [one] Вы падaбралі { $item }
       *[other] Вы падабралі { $amount }x { $item }
    }
# HUD Pickup message
hud-loot-pickup-msg =
    { $gender ->
        [she]
            { $amount ->
                [1] { $actor } падабрала { $item }
               *[other] { $actor } падабрала { $amount }x { $item }
            }
       *[he]
            { $amount ->
                [1] { $actor } падабраў { $item }
               *[other] { $actor } падабраў { $amount }x { $item }
            }
    }
# Npc /tell messages, no gender info, sadly
hud-chat-tell-to-npc = Да [{ $alias }]: { $msg }
# Npc /tell messages, no gender info, sadly
hud-chat-tell-from-npc = Ад [{ $alias }]: { $msg }
# Generic messages
hud-chat-message-with-name = { "[" }{ $alias }] { $name }: { $msg }
# Generic messages
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
# Generic messages
hud-chat-message = { "[" }{ $alias }]: { $msg }
hud-chat-singleplayer-motd1 = Увесь свет для самаго сябе! Час расслабіцца...
hud-chat-singleplayer-motd2 = Як знайсці душэўны спакой?
