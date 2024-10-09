## 玩家事件，$user_gender 可用

hud-chat-online_msg = [{ $name }] 現在上線了。
hud-chat-offline_msg = [{ $name }] 已離線。
hud-chat-goodbye = 再見！
hud-chat-connection_lost = 連線中斷。{ $time } 秒後將踢出。

## 玩家 /tell 消息，$user_gender 可用

hud-chat-tell-to = 給 [{ $alias }]: { $msg }
hud-chat-tell-from = 來自 [{ $alias }]: { $msg }

## NPC /tell 消息，沒有性別信息

hud-chat-tell-to-npc = 給 [{ $alias }]: { $msg }
hud-chat-tell-from-npc = 來自 [{ $alias }]: { $msg }

## 通用消息

hud-chat-message = [{ $alias }]: { $msg }
hud-chat-message-with-name = [{ $alias }] { $name }: { $msg }
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }

## PvP Buff 死亡，$attacker_gender 和 $victim_gender 都可用

hud-chat-died_of_pvp_buff_msg =
 .burning = [{ $victim }] 因 [{ $attacker }] 的灼燒死亡
 .bleeding = [{ $victim }] 因 [{ $attacker }] 的流血死亡
 .curse = [{ $victim }] 因 [{ $attacker }] 的詛咒死亡
 .crippled = [{ $victim }] 因 [{ $attacker }] 的致殘死亡
 .frozen = [{ $victim }] 因 [{ $attacker }] 的冰凍死亡
 .mysterious = [{ $victim }] 因 [{ $attacker }] 的神秘力量死亡

## PvE Buff 死亡，只有 $victim_gender 可用

hud-chat-died_of_npc_buff_msg =
 .burning = [{ $victim }] 因 { $attacker } 的灼燒死亡
 .bleeding = [{ $victim }] 因 { $attacker } 的流血死亡
 .curse = [{ $victim }] 因 { $attacker } 的詛咒死亡
 .crippled = [{ $victim }] 因 { $attacker } 的致殘死亡
 .frozen = [{ $victim }] 因 { $attacker } 的冰凍死亡
 .mysterious = [{ $victim }] 因 { $attacker } 的神秘力量死亡

## 隨機 Buff 死亡，只有 $victim_gender 可用

hud-chat-died_of_buff_nonexistent_msg =
 .burning = [{ $victim }] 因灼燒死亡
 .bleeding = [{ $victim }] 因流血死亡
 .curse = [{ $victim }] 因詛咒死亡
 .crippled = [{ $victim }] 因致殘死亡
 .frozen = [{ $victim }] 因冰凍死亡
 .mysterious = [{ $victim }] 因神秘力量死亡

## 其他 PvP 死亡，$attacker_gender 和 $victim_gender 都可用

hud-chat-pvp_melee_kill_msg = [{ $attacker }] 擊敗了 [{ $victim }]
hud-chat-pvp_ranged_kill_msg = [{ $attacker }] 射殺了 [{ $victim }]
hud-chat-pvp_explosion_kill_msg = [{ $attacker }] 炸死了 [{ $victim }]
hud-chat-pvp_energy_kill_msg = [{ $attacker }] 用魔法殺死了 [{ $victim }]
hud-chat-pvp_other_kill_msg = [{ $attacker }] 殺死了 [{ $victim }]

## 其他 PvE 死亡，只有 $victim_gender 可用

hud-chat-npc_melee_kill_msg = { $attacker } 殺死了 [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } 射殺了 [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } 炸死了 [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } 用魔法殺死了 [{ $victim }]
hud-chat-npc_other_kill_msg = { $attacker } 殺死了 [{ $victim }]

## 其他死亡情況，只有 $victim_gender 可用

hud-chat-fall_kill_msg = [{ $name }] 因墜落傷害死亡
hud-chat-suicide_msg = [{ $name }] 因自殘死亡
hud-chat-default_death_msg = [{ $name }] 死亡

## 聊天工具

hud-chat-all = 全部
hud-chat-chat_tab_hover_tooltip = 右鍵點擊以進行設定

## HUD 撿取消息

hud-loot-pickup-msg-you = { $amount ->
    [1] 你撿起了 { $item }
    *[other] 你撿起了 { $amount }x { $item }
}
hud-loot-pickup-msg = { $amount ->
    [1] { $actor } 撿起了 { $item }
    *[other] { $actor } 撿起了 { $amount }x { $item }
}
