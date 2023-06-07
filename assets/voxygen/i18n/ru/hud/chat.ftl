## Player events
hud-chat-online_msg = [{ $name }] зашёл на сервер
hud-chat-offline_msg = [{ $name }] покинул сервер
## Buff outcomes
hud-outcome-burning = сгорел
hud-outcome-curse = умер от проклятия
hud-outcome-bleeding = умер от кровотечения
hud-outcome-crippled = умер от множественных травм
hud-outcome-frozen = замёрз насмерть
hud-outcome-mysterious = загадочно умер
## Buff deaths
hud-chat-died_of_pvp_buff_msg = [{ $victim }] { $died_of_buff } вызванного [{ $attacker }]
hud-chat-died_of_buff_nonexistent_msg = [{ $victim }] { $died_of_buff }
hud-chat-died_of_npc_buff_msg = [{ $victim }] { $died_of_buff } вызванного { $attacker }
## PvP deaths
hud-chat-pvp_melee_kill_msg = [{ $attacker }] одержал победу над [{ $victim }]
hud-chat-pvp_ranged_kill_msg = [{ $attacker }] застрелил [{ $victim }]
hud-chat-pvp_explosion_kill_msg = [{ $attacker }] взорвал [{ $victim }]
hud-chat-pvp_energy_kill_msg = [{ $attacker }] убил [{ $victim }] магией
hud-chat-pvp_other_kill_msg = [{ $attacker }] убил [{ $victim }]
## PvE deaths
hud-chat-npc_melee_kill_msg = { $attacker } убил [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } застрелил [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } взорвал [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } убил [{ $victim }] магией
hud-chat-npc_other_kill_msg = { $attacker } убил [{ $victim }]
## Other deaths
hud-chat-environmental_kill_msg = [{ $name }] умер в { $environment }
hud-chat-fall_kill_msg = [{ $name }] умер от падения
hud-chat-suicide_msg = [{ $name }] умер от ран, нанесённых самому себе
hud-chat-default_death_msg = [{ $name }] умер
## Utils
hud-chat-all = Все
hud-chat-you = Вы
hud-chat-mod = Модератор
hud-chat-chat_tab_hover_tooltip = ПКМ для настроек
hud-loot-pickup-msg = {$actor} подобрал { $amount ->
   [one] { $item }
   *[other] {$amount}x {$item}
}
hud-chat-loot_fail = Ваш инвентарь полон!
hud-chat-goodbye = До свидания!
hud-chat-connection_lost = Соединение потеряно. Выход через { $time } секунд.