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
