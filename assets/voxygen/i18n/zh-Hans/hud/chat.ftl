hud-chat-all = 全部
hud-chat-chat_tab_hover_tooltip = 右键单击设置
hud-chat-online_msg = { "[" }{ $name }] 上线了。
hud-chat-offline_msg = { "[" }{ $name }] 下线了。
hud-chat-default_death_msg = { "[" }{ $name }]死了
hud-chat-fall_kill_msg = { "[" }{ $name }]因摔落伤害而死亡
hud-chat-suicide_msg = { "[" }{ $name }]因自伤而死亡
hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $victim }] 死于：燃烧 由于 [{ $attacker }]
    .bleeding = { "[" }{ $victim }] 死于：流血 由于 [{ $attacker }]
    .curse = { "[" }{ $victim }] 死于：诅咒 由于 [{ $attacker }]
    .crippled = { "[" }{ $victim }] 死于：残废 由于 [{ $attacker }]
    .frozen = { "[" }{ $victim }] 死于：冻结 由于 [{ $attacker }]
    .mysterious = { "[" }{ $victim }] 死于：神秘（不明） 由于 [{ $attacker }]
hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }]击败了[{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }]射杀了[{ $victim }]
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }]炸死了[{ $victim }]
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }]用魔法击杀了[{ $victim }]
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }]杀死了[{ $victim }]
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] 死于：燃烧
    .bleeding = { "[" }{ $victim }] 死于：流血
    .curse = { "[" }{ $victim }] 死于：诅咒
    .crippled = { "[" }{ $victim }] 死于：残废
    .frozen = { "[" }{ $victim }] 死于：冻结
    .mysterious = { "[" }{ $victim }] 死于：神秘（不明）
hud-chat-died_of_npc_buff_msg =
    .burning = { "[" }{ $victim }] 死于：燃烧 由于 { $attacker }
    .bleeding = { "[" }{ $victim }] 死于：流血 由于 { $attacker }
    .curse = { "[" }{ $victim }] 死于：诅咒 由于 { $attacker }
    .crippled = { "[" }{ $victim }] 死于：残废 由于 { $attacker }
    .frozen = { "[" }{ $victim }] 死于：冻结 由于 { $attacker }
    .mysterious = { "[" }{ $victim }] 死于：神秘（不明） 由于 { $attacker }
hud-chat-npc_melee_kill_msg = { $attacker }击杀了[{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker }射杀了[{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker }炸死了[{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker }用魔法击杀了[{ $victim }]
hud-chat-npc_other_kill_msg = { $attacker }击杀了[{ $victim }]
hud-loot-pickup-msg =
    { $amount ->
        [1] { $actor } 拾取了 { $item }
       *[other] { $actor } 拾取了 { $amount }x { $item }
    }
hud-chat-goodbye = 再见！
hud-chat-connection_lost = 连接中断。{ $time }秒后将被踢出。
hud-chat-tell-to-npc = 向 [{ $alias }] 说：{ $msg }
hud-chat-tell-from-npc = 来自[{ $alias }]：{ $msg }
hud-chat-tell-to = 致 [{ $alias }]：{ $msg }
hud-chat-tell-from = 来自 [{ $alias }]：{ $msg }
hud-chat-message = { "[" }{ $alias }]：{ $msg }
hud-chat-message-with-name = { "[" }{ $alias }] { $name }：{ $msg }
hud-chat-message-in-group = ({ $group }) [{ $alias }]：{ $msg }
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }：{ $msg }
hud-loot-pickup-msg-you =
    { $amount ->
        [1] 你拾取了 { $item }
       *[other] 你拾取了 { $amount }x { $item }
    }
hud-chat-singleplayer-motd1 = 一整个世界都属于你！是时候大展身手了……
hud-chat-singleplayer-motd2 = 这片宁静如何？
