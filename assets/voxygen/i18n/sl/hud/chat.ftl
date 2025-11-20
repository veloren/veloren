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
