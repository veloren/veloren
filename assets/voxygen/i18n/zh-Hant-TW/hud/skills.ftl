## Internal terms, currently only used in es
## 如果從這裡刪除，它們也會自動從es刪除，
## 所以即使在英文文件中沒有使用，也請保留它們。
## 請參閱 https://github.com/WeblateOrg/weblate/issues/9895
-hud-skill-sc_wardaura_title = ""
-hud-skill-bow_shotgun_title = ""
-hud-skill-st_shockwave_title = ""

## 技能樹 UI

hud-skill_tree-general = 一般戰鬥
hud-skill_tree-sword = 劍
hud-skill_tree-axe = 斧
hud-skill_tree-hammer = 槌
hud-skill_tree-bow = 弓
hud-skill_tree-staff = 火焰法杖
hud-skill_tree-sceptre = 權杖
hud-skill_tree-mining = 採礦

hud-rank_up = 新技能點數
hud-skill-sp_available =
    { $number ->
        [0] 無技能點數可用
        [1] { $number } 點技能點數可用
        *[other] { $number } 點技能點數可用
    }
hud-skill-not_unlocked = 尚未解鎖
hud-skill-req_sp = {"\u000A"}需要 { $number } SP
hud-skill-set_as_exp_bar = 在經驗值條上跟蹤進度

hud-skill-unlck_sword_title = 劍精通
hud-skill-unlck_sword = 解鎖劍技能樹。{ $SP }
hud-skill-unlck_axe_title = 斧精通
hud-skill-unlck_axe = 解鎖斧技能樹。{ $SP }
hud-skill-unlck_hammer_title = 槌精通
hud-skill-unlck_hammer = 解鎖槌技能樹。{ $SP }
hud-skill-unlck_bow_title = 弓精通
hud-skill-unlck_bow = 解鎖弓技能樹。{ $SP }
hud-skill-unlck_staff_title = 法杖精通
hud-skill-unlck_staff = 解鎖法杖技能樹。{ $SP }
hud-skill-unlck_sceptre_title = 權杖精通
hud-skill-unlck_sceptre = 解鎖權杖技能樹。{ $SP }
hud-skill-climbing_title = 攀爬
hud-skill-climbing = 能夠攀爬表面。
hud-skill-climbing_cost_title = 攀爬消耗
hud-skill-climbing_cost = 攀爬使用 { $boost } % 較少的能量。{ $SP }
hud-skill-climbing_speed_title = 攀爬速度
hud-skill-climbing_speed = 攀爬速度提升 { $boost } %。{ $SP }
hud-skill-swim_title = 游泳
hud-skill-swim = 在水中移動。
hud-skill-swim_speed_title = 游泳速度
hud-skill-swim_speed = 游泳速度提升 { $boost } %。{ $SP }
hud-skill-sc_lifesteal_title = 吸血光束
hud-skill-sc_lifesteal = 從敵人那裡吸取生命。
hud-skill-sc_lifesteal_damage_title = 傷害
hud-skill-sc_lifesteal_damage = 傷害增加 { $boost } %。{ $SP }
hud-skill-sc_lifesteal_range_title = 範圍
hud-skill-sc_lifesteal_range = 你的光束延伸 { $boost } % 更遠。{ $SP }
hud-skill-sc_lifesteal_lifesteal_title = 吸血
hud-skill-sc_lifesteal_lifesteal = 將額外 { $boost } % 的傷害轉化為生命值。{ $SP }
hud-skill-sc_lifesteal_regen_title = 能量再生
hud-skill-sc_lifesteal_regen = 額外補充 { $boost } % 的能量。{ $SP }
hud-skill-sc_heal_title = 治療光環
hud-skill-sc_heal = 使用敵人的血液治療你的盟友，需要連擊才能激活。
hud-skill-sc_heal_heal_title = 治療
hud-skill-sc_heal_heal = 治療量增加 { $boost } %。{ $SP }
hud-skill-sc_heal_cost_title = 能量消耗
hud-skill-sc_heal_cost = 治療消耗降低 { $boost } %。{ $SP }
hud-skill-sc_heal_duration_title = 持續時間
hud-skill-sc_heal_duration = 治療光環的效果持續 { $boost } % 更久。{ $SP }
hud-skill-sc_heal_range_title = 範圍
hud-skill-sc_heal_range = 治療光環的範圍增加 { $boost } %。{ $SP }
hud-skill-sc_wardaura_unlock_title = 防護光環解鎖
hud-skill-sc_wardaura_unlock = 允許你為盟友抵擋敵方攻擊。{ $SP }
hud-skill-sc_wardaura_strength_title = 強度
hud-skill-sc_wardaura_strength = 防護效果增加 { $boost } %。{ $SP }
hud-skill-sc_wardaura_duration_title = 持續時間
hud-skill-sc_wardaura_duration = 防護效果持續 { $boost } % 更久。{ $SP }
hud-skill-sc_wardaura_range_title = 範圍
hud-skill-sc_wardaura_range = 防護範圍增加 { $boost } %。{ $SP }
hud-skill-sc_wardaura_cost_title = 能量消耗
hud-skill-sc_wardaura_cost = 創建防護所需的能量減少 { $boost } %。{ $SP }
hud-skill-st_shockwave_range_title = 衝擊波範圍
hud-skill-st_shockwave_range = 投擲遠距離物體，範圍增加 { $boost } %。{ $SP }
hud-skill-st_shockwave_cost_title = 衝擊波消耗
hud-skill-st_shockwave_cost = 減少投擲能量消耗 { $boost } %。{ $SP }
hud-skill-st_shockwave_knockback_title = 衝擊波擊退
hud-skill-st_shockwave_knockback = 擊退力增加 { $boost } %。{ $SP }
hud-skill-st_shockwave_damage_title = 衝擊波傷害
hud-skill-st_shockwave_damage = 傷害增加 { $boost } %。{ $SP }
hud-skill-st_shockwave_unlock_title = 衝擊波解鎖
hud-skill-st_shockwave_unlock = 解鎖使用火焰擊退敵人的能力。{ $SP }
hud-skill-st_flamethrower_title = 火焰噴射器
hud-skill-st_flamethrower = 噴射火焰，把他們全烤熟。
hud-skill-st_flame_velocity_title = 火焰速度
hud-skill-st_flame_velocity = 火焰移動速度提升 { $boost } %。{ $SP }
hud-skill-st_flamethrower_range_title = 火焰噴射器範圍
hud-skill-st_flamethrower_range = 當火焰範圍不夠時，它將延伸 { $boost } % 更遠。{ $SP }
hud-skill-st_energy_drain_title = 能量消耗
hud-skill-st_energy_drain = 能量消耗速率降低 { $boost } %。{ $SP }
hud-skill-st_flamethrower_damage_title = 火焰噴射器傷害
hud-skill-st_flamethrower_damage = 傷害增加 { $boost } %。{ $SP }
hud-skill-st_explosion_radius_title = 爆炸半徑
hud-skill-st_explosion_radius = 範圍擴大，爆炸半徑增加 { $boost } %。{ $SP }
hud-skill-st_energy_regen_title = 能量再生
hud-skill-st_energy_regen = 能量回復速度增加 { $boost } %。{ $SP }
hud-skill-st_fireball_title = 火球
hud-skill-st_fireball = 射出一顆火球，碰撞後爆炸。
hud-skill-st_damage_title = 傷害
hud-skill-st_damage = 傷害增加 { $boost } %。{ $SP }
hud-skill-bow_projectile_speed_title = 射擊速度
hud-skill-bow_projectile_speed = 射箭速度加快 { $boost } %。{ $SP }
hud-skill-bow_charged_title = 蓄力射擊
hud-skill-bow_charged = 因為你蓄力時間更久。
hud-skill-bow_charged_damage_title = 蓄力傷害
hud-skill-bow_charged_damage = 傷害增加 { $boost } %。{ $SP }
hud-skill-bow_charged_energy_regen_title = 蓄力回復
hud-skill-bow_charged_energy_regen = 能量回復速度增加 { $boost } %。{ $SP }
hud-skill-bow_charged_knockback_title = 蓄力擊退
hud-skill-bow_charged_knockback = 擊退距離增加 { $boost } %。{ $SP }
hud-skill-bow_charged_speed_title = 蓄力速度
hud-skill-bow_charged_speed = 蓄力攻擊的速度增加 { $boost } %。{ $SP }
hud-skill-bow_charged_move_title = 蓄力移動速度
hud-skill-bow_charged_move = 蓄力攻擊時移動速度提升 { $boost } %。{ $SP }
hud-skill-bow_repeater_title = 重複射擊
hud-skill-bow_repeater = 發射越久，射擊越快。
hud-skill-bow_repeater_damage_title = 重複射擊傷害
hud-skill-bow_repeater_damage = 傷害增加 { $boost } %。{ $SP }
hud-skill-bow_repeater_cost_title = 重複射擊消耗
hud-skill-bow_repeater_cost = 減少重複射擊的能量消耗 { $boost } %。{ $SP }
hud-skill-bow_repeater_speed_title = 重複射擊速度
hud-skill-bow_repeater_speed = 射箭速度增加 { $boost } %。{ $SP }
hud-skill-bow_shotgun_unlock_title = 解鎖霰彈
hud-skill-bow_shotgun_unlock = 解鎖一次發射多支箭的能力。{ $SP }
hud-skill-bow_shotgun_damage_title = 霰彈傷害
hud-skill-bow_shotgun_damage = 傷害增加 { $boost } %。{ $SP }
hud-skill-bow_shotgun_cost_title = 霰彈消耗
hud-skill-bow_shotgun_cost = 減少霰彈的能量消耗 { $boost } %。{ $SP }
hud-skill-bow_shotgun_arrow_count_title = 霰彈箭數
hud-skill-bow_shotgun_arrow_count = 增加箭數 { $boost }。{ $SP }
hud-skill-bow_shotgun_spread_title = 霰彈擴散
hud-skill-bow_shotgun_spread = 減少箭的擴散範圍 { $boost } %。{ $SP }
hud-skill-mining_title = 採礦
hud-skill-pick_strike_title = 鑿擊
hud-skill-pick_strike = 用鎬子敲打岩石以獲取礦石、寶石和經驗值。
hud-skill-pick_strike_speed_title = 鑿擊速度
hud-skill-pick_strike_speed = 挖礦速度提升。{ $SP }
hud-skill-pick_strike_oregain_title = 鑿擊礦石收獲
hud-skill-pick_strike_oregain = 額外獲得礦石的機會（每級 { $boost } %）。{ $SP }
hud-skill-pick_strike_gemgain_title = 鑿擊寶石收獲
hud-skill-pick_strike_gemgain = 額外獲得寶石的機會（每級 { $boost } %）。{ $SP }

## 技能樹錯誤對話框
hud-skill-persistence-hash_mismatch = 自上次遊玩以來，檢測到其中一個技能組發生差異
hud-skill-persistence-deserialization_failure = 從數據庫加載技能時發生錯誤
hud-skill-persistence-spent_experience_missing = 其中一個技能組中的可用經驗值數量與你上次遊玩時不同
hud-skill-persistence-skills_unlock_failed = 未能以你獲取技能的相同順序解鎖技能，前置條件或成本可能已更改
hud-skill-persistence-common_message = 你的一些技能點數已被重置，請重新分配
