## Player events
hud-chat-online_msg = [{ $name }] { $user_gender ->
    [she] зайшла на сервер
    *[he] зайшов на сервер
}
hud-chat-offline_msg = [{ $name }] { $user_gender ->
    [she] вийшла з серверу
    *[he] вийшов з серверу
}
hud-chat-goodbye = До побачення!
hud-chat-connection_lost = З'єднання втрачено. Відключення через { $time ->
    [one] { $time } секунду
    [few] { $time } секунди
    *[other] { $time } секунд
}

## PvP Buff deaths
hud-chat-died_of_pvp_buff_msg =
    .burning = { $attacker_gender ->
        [she] [{ $attacker }] спалила [{ $victim }] живцем
        *[he] [{ $attacker }] спалив [{ $victim }] живцем
    }
    .bleeding = { $victim_gender ->
        [she] [{$victim}] втратила занадто багато крові отримавши поранення від [{ $attacker }]
        *[he] [{$victim}] втратив занадто багато крові отримавши поранення від [{ $attacker }]
    }
    .curse = { $victim_gender ->
        [she] [{ $victim }] померла від прокляття накладеного [{ $attacker }]
        *[he] [{ $victim }] помер від прокляття накладеного [{ $attacker }]
    }
    .crippled = { $victim_gender ->
        [she] [{ $victim }] загинула від отриманих травм через [{ $attacker }]
        *[he] [{ $victim }] загинув від отриманих травм через [{ $attacker }]
    }
    .frozen = { $victim_gender ->
        [she] [{ $victim }] замерзла на смерть через [{ $attacker }]
        *[he] [{ $victim }] замерз на смерть через [{ $attacker }]
    }
    .mysterious = { $victim_gender ->
        [she] [{ $victim }] померла ... через [{ $attacker }] ... як?
        *[he] [{ $victim }] помер ... через [{ $attacker }] ... як?
    }

## PvE buff deaths
hud-chat-died_of_npc_buff_msg =
    .burning = { $victim_gender ->
        [she] [{ $victim }] отримала через [{ $attacker }] опіки несумісні з життям
        *[he] [{ $victim }] отримав через [{ $attacker }] опіки несумісні з життям
    }
    .bleeding = { $victim_gender ->
        [she] [{ $victim }] втратила занадто багато крові отримавши поранення від [{ $attacker }]
        *[he] [{ $victim }] втратив занадто багато крові отримавши поранення від [{ $attacker }]
    }
    .curse = { $victim_gender ->
        [she] [{ $victim }] померла від прокляття накладеного [{ $attacker }]
        *[he] [{ $victim }] помер від прокляття накладеного [{ $attacker }]
    }
    .crippled = { $victim_gender ->
        [she] [{ $victim }] загинула від отриманих травм через [{ $attacker }]
        *[he] [{ $victim }] загинув від отриманих травм через [{ $attacker }]
    }
    .frozen = { $victim_gender ->
        [she] [{ $victim }] замерзла на смерть через [{ $attacker }]
        *[he] [{ $victim }] замерз на смерть через [{ $attacker }]
    }
    .mysterious = { $victim_gender ->
        [she] [{ $victim }] померла ... через [{ $attacker }] ... як?
        *[he] [{ $victim }] помер ... через [{ $attacker }] ... як?
    }

## Random buff deaths
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { $victim_gender ->
        [she] [{ $victim }] отримала опіки несумісні з життям
        *[he] [{ $victim }] отримав опіки несумісні з життям
    }
    .bleeding = { $victim_gender ->
        [she] [{ $victim }] втратила занадто багато крові
        *[he] [{ $victim }] втратив занадто багато крові
    }
    .curse = { $victim_gender ->
        [she] [{ $victim }] померла від прокляття
        *[he] [{ $victim }] помер від прокляття
    }
    .crippled = { $victim_gender ->
        [she] [{ $victim }] загинула від отриманих травм
        *[he] [{ $victim }] загинув від отриманих травм
    }
    .frozen = { $victim_gender ->
        [she] [{ $victim }] замерзла на смерть
        *[he] [{ $victim }] замерз на смерть
    }
    .mysterious = { $victim_gender ->
        [she] [{ $victim }] померла ... як?
        *[he] [{ $victim }] помер ... як?
    }

## PvP deaths
hud-chat-pvp_melee_kill_msg = { $victim_gender ->
    [she] [{ $victim }] вбита [{ $attacker }] у ближньому двобої
    *[he] [{ $victim }] вбитий [{ $attacker }] у ближньому двобої
}
hud-chat-pvp_ranged_kill_msg = { $victim_gender ->
    [she] [{ $victim }] впала вбита після влучного пострілу [{ $attacker }]
    *[he] [{ $victim }] впав вбитий після влучного пострілу [{ $attacker }]
}
hud-chat-pvp_explosion_kill_msg = { $victim_gender ->
    [she] [{ $victim }] розлетілась на атоми від удару [{ $attacker }]
    *[he] [{ $victim }] розлетівся на атоми від удару [{ $attacker }]
}
hud-chat-pvp_energy_kill_msg = { $victim_gender ->
    [she] [{ $victim }] впала вбита не встигнувши ухилитися від енергетичної атаки [{ $attacker }]
    *[he] [{ $victim }] впав вбитий не встигнувши ухилитися від енергетичної атаки [{ $attacker }]
}
hud-chat-pvp_other_kill_msg = { $victim_gender ->
    [she] [{ $victim }] вбита [{ $attacker }]
    *[he] [{ $victim }] вбитий [{ $attacker }]
}

## PvE deaths
hud-chat-npc_melee_kill_msg = { $victim_gender ->
    [she] [{ $victim }] вбита [{ $attacker }] у ближньому двобої
    *[he] [{ $victim }] вбитий [{ $attacker }] у ближньому двобої
}
hud-chat-npc_ranged_kill_msg = { $victim_gender ->
    [she] [{ $victim }] впала вбита після влучного пострілу [{ $attacker }]
    *[he] [{ $victim }] впав вбитий після влучного пострілу [{ $attacker }]
}
hud-chat-npc_explosion_kill_msg = { $victim_gender ->
    [she] [{ $victim }] розлетілась на атоми від удару [{ $attacker }]
    *[he] [{ $victim }] розлетівся на атоми від удару [{ $attacker }]
}
hud-chat-npc_energy_kill_msg = { $victim_gender ->
    [she] [{ $victim }] впала вбита не встигнувши ухилитися від енергетичної атаки [{ $attacker }]
    *[he] [{ $victim }] впав вбитий не встигнувши ухилитися від енергетичної атаки [{ $attacker }]
}
hud-chat-npc_other_kill_msg = { $victim_gender ->
    [she] [{ $victim }] вбита [{ $attacker }]
    *[he] [{ $victim }] вбитий [{ $attacker }]
}

## Other deaths

hud-chat-fall_kill_msg = { $victim_gender ->
    [she] [{ $name }] померла від падіння
    *[he] [{ $name }] помер від падіння
}
hud-chat-suicide_msg = { $victim_gender ->
    [she] [{ $name }] померла від самозаподіяних ран
    *[he] [{ $name }] помер від самозаподіяних ран
}
hud-chat-default_death_msg = { $victim_gender ->
    [she] [{ $name }] померла
    *[he] [{ $name }] помер
}

## Chat utils

hud-chat-all = Все
hud-chat-chat_tab_hover_tooltip = Правий клік для налаштування

# hud-chat-you = Ви

## HUD Pickup message

hud-loot-pickup-msg = { $is_you ->
    [true] Ви підняли { $amount ->
        [1] { $item }
        *[other] {$amount}x {$item}
    }
    *[false] { $gender ->
        [she] { $actor } підняла { $amount ->
            [1] { $item }
            *[other] { $amount }x { $item }
        }
        [he] { $actor } підняв { $amount ->
            [1] { $item }
            *[other] { $amount }x { $item }
        }
        *[other] { $actor } підняло { $amount ->
            [1] { $item }
            *[other] { $amount }x { $item }
        }
    }
}