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
hud-chat-fall_kill_msg = { "[" }{ $name }] died from fall damage
hud-chat-suicide_msg = { "[" }{ $name }] died from self-inflicted wounds
hud-chat-default_death_msg = { "[" }{ $name }] died
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
