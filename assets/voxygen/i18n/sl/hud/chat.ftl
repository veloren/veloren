hud-chat-online_msg =
    { $user_gender ->
        [she] { "[" }{ $name }] se je pridružila.
       *[he] { "[" }{ $name }] se je pridružil.
    }
hud-chat-offline_msg =
    { $user_gender ->
        [she] { "[" }{ $name }] se je odklopila.
       *[he] { "[" }{ $name }] se je odklopil.
    }
hud-chat-goodbye = Zbogom!
hud-chat-connection_lost =
    Povezava prekinjena. { $time ->
        [one] Čez { $time } sekundo te bo vrglo iz igre.
        [two] Čez { $time } sekundi te bo vrglo iz igre.
       *[other] Čez { $time } sekund te bo vrglo iz igre.
    }
hud-chat-tell-to = Za [{ $alias }]: { $msg }
hud-chat-tell-from = Od [{ $alias }]: { $msg }
hud-chat-tell-to-npc = Za [{ $alias }]: { $msg }
hud-chat-tell-from-npc = Od [{ $alias }]: { $msg }
hud-chat-message = { "[" }{ $alias }]: { $msg }
hud-chat-message-with-name = { "[" }{ $alias }] { $name }: { $msg }
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }
hud-chat-npc_melee_kill_msg = { $attacker } je ubil_a [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } je ustrelil_a [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } je razstrelil_a [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } je ubil_a [{ $victim }] s čarovnijo
hud-chat-npc_other_kill_msg = { $attacker } je ubil_a [{ $victim }]
hud-chat-fall_kill_msg =
    { $victim_gender ->
        [she] { "[" }{ $name }] je padla do smrti
        [he] { "[" }{ $name }] je padel do smrti
       *[other] { "[" }{ $name }] je padlo do smrti
    }
hud-chat-suicide_msg =
    { $victim_gender ->
        [she] { "[" }{ $name }] je podlegla samopoškodbi
        [he] { "[" }{ $name }] je podlegel samopoškodbi
       *[other] { "[" }{ $name }] je podleglo samopoškodbi
    }
hud-chat-default_death_msg =
    { $victim_gender ->
        [she] { "[" }{ $name }] je umrla
        [he] { "[" }{ $name }] je umrl
       *[other] { "[" }{ $name }] je umrlo
    }
hud-chat-all = Vse
hud-chat-chat_tab_hover_tooltip = Desni klik za nastavitve
hud-loot-pickup-msg-you =
    { $amount ->
        [1] Zdaj imaš { $item }
       *[other] Zdaj imaš { $amount }x { $item }
    }
hud-loot-pickup-msg =
    { $gender ->
        [she]
            { $amount ->
                [1] { $actor } je pobrala { $item }
               *[other] { $actor } je pobrala { $amount }x { $item }
            }
       *[he]
            { $amount ->
                [1] { $actor } je pobral { $item }
               *[other] { $actor } je pobral { $amount }x { $item }
            }
    }
hud-chat-singleplayer-motd1 = Ves svet imaš samo zase! Pretegni si noge ...
hud-chat-singleplayer-motd2 = Mir in tišina ...
hud-chat-died_of_pvp_buff_msg =
    .burning =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je umrla: [{ $attacker }] jo je
           *[he] { "[" }{ $victim }] je umrl: [{ $attacker }] ga je
        } { $attacker_gender ->
            [she] zažgala.
           *[he] zažgal.
        }
    .bleeding =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je izkrvavela.
           *[he] { "[" }{ $victim }] je izkrvavel.
        } { $attacker_gender ->
            [she] Kriva je [{ $attacker }].
           *[he] Kriv je [{ $attacker }].
        }
    .curse =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je podlegla prekletstvu,
           *[he] { "[" }{ $victim }] je podlegel prekletstvu,
        } { $attacker_gender ->
            [she] ki ga je povzročila [{ $attacker }].
           *[he] ki ga je povzročil [{ $attacker }].
        }
    .crippled =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je umrla zaradi hudih poškodb,
           *[he] { "[" }{ $victim }] je umrl zaradi hudih poškodb,
        } { $attacker_gender ->
            [she] ki jih je povzročila [{ $attacker }].
           *[he] ki jih je povzročil [{ $attacker }].
        }
    .frozen =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je zmrznila do smrti,
           *[he] { "[" }{ $victim }] je zmrznil do smrti,
        } { $attacker_gender ->
            [she] kriva je [{ $attacker }].
           *[he] kriv je [{ $attacker }].
        }
    .mysterious =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je umrla skrivnostne smrti,
           *[he] { "[" }{ $victim }] je umrl skrivnostne smrti,
        } { $attacker_gender ->
            [she] ki jo je povzročila [{ $attacker }].
           *[he] ki jo je povzročil [{ $attacker }].
        }
hud-chat-died_of_npc_buff_msg =
    .burning =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je zgorela, kriv_a je  { $attacker }.
           *[he] { "[" }{ $victim }] je zgorel, kriv_a je { $attacker }.
        }
    .bleeding =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je izkrvavela, kriv_a je  { $attacker }.
           *[he] { "[" }{ $victim }] je izkrvavel, kriv_a je { $attacker }.
        }
    .curse =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je podlegla prekletstvu, kriv_a je  { $attacker }.
           *[he] { "[" }{ $victim }] je podlegel prekletstvu, kriv_a je { $attacker }.
        }
    .crippled =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je umrla zaradi hudih poškodb, kriv_a je  { $attacker }.
           *[he] { "[" }{ $victim }] je umrl zaradi hudih poškodb, kriv_a je { $attacker }.
        }
    .frozen =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je zmrznila, kriv_a je  { $attacker }.
           *[he] { "[" }{ $victim }] je zmrznil, kriv_a je { $attacker }.
        }
    .mysterious =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je umrla skrivnostne smrti, kriv_a je  { $attacker }.
           *[he] { "[" }{ $victim }] je umrl skrivnostne smrti, kriv_a je { $attacker }.
        }
hud-chat-died_of_buff_nonexistent_msg =
    .burning =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je zgorela
           *[he] { "[" }{ $victim }] je zgorel
        }
    .bleeding =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je izkrvavela
           *[he] { "[" }{ $victim }] je izkrvavel
        }
    .curse =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je podlegla prekletstvu
           *[he] { "[" }{ $victim }] je podlegel prekletstvu
        }
    .crippled =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je umrla od hudih poškodb
           *[he] { "[" }{ $victim }] je umrl od hudih poškodb
        }
    .frozen =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je zmrznila
           *[he] { "[" }{ $victim }] je zmrznil
        }
    .mysterious =
        { $victim_gender ->
            [she] { "[" }{ $victim }] je umrla skrivnostne smrti
           *[he] { "[" }{ $victim }] je umrl skrivnostne smrti
        }
hud-chat-pvp_melee_kill_msg =
    { $attacker_gender ->
        [she] { "[" }{ $attacker }] je porazila [{ $victim }]
       *[he] { "[" }{ $attacker }] je porazil [{ $victim }]
    }
hud-chat-pvp_ranged_kill_msg =
    { $attacker_gender ->
        [she] { "[" }{ $attacker }] je ustrelila [{ $victim }]
       *[he] { "[" }{ $attacker }] je ustrelil [{ $victim }]
    }
hud-chat-pvp_explosion_kill_msg =
    { $attacker_gender ->
        [she] { "[" }{ $attacker }] je razstrelila [{ $victim }]
       *[he] { "[" }{ $attacker }] je ustrelil [{ $victim }]
    }
hud-chat-pvp_energy_kill_msg =
    { $attacker_gender ->
        [she] { "[" }{ $attacker }] je ubila [{ $victim }] s čarovnijo
       *[he] { "[" }{ $attacker }] je ubil [{ $victim }] s čarovnijo
    }
hud-chat-pvp_other_kill_msg =
    { $attacker_gender ->
        [she] { "[" }{ $attacker }] je ubila [{ $victim }]
       *[he] { "[" }{ $attacker }] je ubil [{ $victim }]
    }
