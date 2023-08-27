## Player events
hud-chat-online_msg = [{ $name }] зайшов/-ла на сервер
hud-chat-offline_msg = [{ $name }] вийшов/-ла з серверу
## Buff deaths
hud-chat-died_of_pvp_buff_msg =
 .burning = [{ $victim }] згорів/-ла живцем через [{ $attacker }]
 .bleeding = [{ $victim }] помер/-ла від кровотечі через [{ $attacker }]
 .curse = [{ $victim }] помер/-ла від прокльону через [{ $attacker }]
 .crippled = [{ $victim }] загинув/-ла від травм через [{ $attacker }]
 .frozen = [{ $victim }] замерз/-ла на смерть через [{ $attacker }]
 .mysterious = [{ $victim }] помер/-ла таємничою смертю через [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg =
 .burning = [{ $victim }] згорів/-ла живцем
 .bleeding = [{ $victim }] помер/-ла від кровотечі
 .curse = [{ $victim }] помер/-ла від прокльону
 .crippled = [{ $victim }] загинув/-ла від травм
 .frozen = [{ $victim }] замерз/-ла на смерть
 .mysterious = [{ $victim }] помер/-ла таємничою смертю
hud-chat-died_of_npc_buff_msg =
 .burning = [{ $victim }] згорів/-ла живцем через { $attacker }
 .bleeding = [{ $victim }] помер/-ла від кровотечі через { $attacker }
 .curse = [{ $victim }] помер/-ла від прокльону через { $attacker }
 .crippled = [{ $victim }] загинув/-ла від травм через { $attacker }
 .frozen = [{ $victim }] замерз/-ла на смерть через { $attacker }
 .mysterious = [{ $victim }] помер/-ла таємничою смертю через { $attacker }
## PvP deaths
hud-chat-pvp_melee_kill_msg = [{ $attacker }] переміг/-ла [{ $victim }]
hud-chat-pvp_ranged_kill_msg = [{ $attacker }] застрелив/-ла [{ $victim }]
hud-chat-pvp_explosion_kill_msg = [{ $attacker }] підірвав/-ла [{ $victim }]
hud-chat-pvp_energy_kill_msg = [{ $attacker }] вбив/-ла [{ $victim }] магією
hud-chat-pvp_other_kill_msg = { $attacker } вбив/-ла [{ $victim }]
## PvE deaths
hud-chat-npc_melee_kill_msg = { $attacker } вбив/-ла [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } застрелив/-ла [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } підірвав/-ла [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } вбив/-ла [{ $victim }] магією
hud-chat-npc_other_kill_msg = { $attacker } вбив/-ла [{ $victim }]
## Other deaths
hud-chat-environmental_kill_msg = [{ $name }] помер/-ла в { $environment }
hud-chat-fall_kill_msg = [{ $name }] помер/-ла від падіння
hud-chat-suicide_msg = [{ $name }] помер/-ла від самозаподіяних ран
hud-chat-default_death_msg = [{ $name }] помер/-ла
## Utils
hud-chat-all = Усі
hud-chat-you = Ти
hud-chat-chat_tab_hover_tooltip = Правий клік для налаштування
hud-loot-pickup-msg = {$actor} підняли { $amount ->
   [1] { $item }
   *[other] {$amount}x {$item}
}
hud-chat-loot_fail = Ваш інвентар переповнено!
hud-chat-goodbye = До побачення!
hud-chat-connection_lost = З'єднання втрачено. Перепідключення через { $time ->
    [one] { $time } секунду
    [few] { $time } секунди
    *[other] { $time } секунд
}