# 描述與幫助

command-help-template = { $usage } { $description }
command-help-additional-shortcuts = 此外，您可以使用以下快捷鍵：

## 伺服器指令

command-adminify-desc = 臨時賦予玩家有限的管理員角色，或移除當前角色（如果沒有賦予）
command-airship-desc = 生成飛空艇
command-alias-desc = 更改您的別名
command-area_add-desc = 新增一個建築區域
command-area_list-desc = 列出所有建築區域
command-area_remove-desc = 移除指定的建築區域
command-aura-desc = 創建一個光環
command-body-desc = 將您的角色變成不同種族
command-buff-desc = 給玩家施加增益效果
command-build-desc = 開關建築模式
command-ban-desc = 封禁某個使用者名稱的玩家，提供時限可設定封禁時間，如果需要覆寫現有封禁請傳遞 true
command-battlemode-desc = 設置戰鬥模式：
  + pvp（玩家對玩家）
  + pve（玩家對環境）
  如果不帶參數將顯示當前的戰鬥模式
command-battlemode_force-desc = 在不進行任何檢查的情況下更改戰鬥模式標誌
command-campfire-desc = 生成營火
command-clear_persisted_terrain-desc = 清除附近的持久化地形
command-create_location-desc = 在當前位置創建地點
command-death_effect-dest = 添加一個死亡效果到目標實體
command-debug_column-desc = 打印列的調試信息
command-debug_ways-desc = 打印列的方式的調試信息
command-delete_location-desc = 刪除某個地點
command-destroy_tethers-desc = 摧毀所有連接到您的繫繩
command-disconnect_all_players-desc = 斷開所有玩家的伺服器連線
command-dismount-desc = 如果您正在騎乘，則下馬；如果有其他東西騎乘您，則將其解下
command-dropall-desc = 將您所有的物品丟在地上
command-dummy-desc = 生成一個訓練假人
command-explosion-desc = 讓地面爆炸
command-faction-desc = 向您的派系發送訊息
command-give_item-desc = 給自己一些物品，使用 Tab 鍵獲取示例或自動完成
command-goto-desc = 傳送到某個位置
command-group-desc = 向您的群組發送訊息
command-group_invite-desc = 邀請玩家加入群組
command-group_kick-desc = 從群組中移除玩家
command-group_leave-desc = 離開當前群組
command-group_promote-desc = 提升某玩家為群組領導者
command-health-desc = 設置您當前的生命值
command-into_npc-desc = 將自己轉換為 NPC，請謹慎使用！
command-join_faction-desc = 加入/離開指定的派系
command-jump-desc = 偏移您當前的位置
command-kick-desc = 踢出某個使用者名稱的玩家
command-kill-desc = 自殺
command-kill_npcs-desc = 殺死 NPC
command-kit-desc = 將一組物品放入您的物品欄
command-lantern-desc = 更改您的燈籠強度和顏色
command-light-desc = 生成具有光線的實體
command-lightning-desc = 在當前位置放出閃電
command-location-desc = 傳送到某個地點
command-make_block-desc = 在您的位置生成一個具有顏色的方塊
command-make_npc-desc = 根據配置在您附近生成實體
  使用 Tab 鍵獲取示例或自動完成
command-make_sprite-desc = 在您位置生成一個精靈
command-make_volume-desc = 創建一個體積（實驗性）
command-motd-desc = 查看伺服器描述
command-mount-desc = 騎上某個實體
command-object-desc = 生成一個物件
command-outcome-desc = 創建一個結果
command-permit_build-desc = 給予玩家在某範圍內建造的權限
command-players-desc = 列出當前在線的玩家
command-portal-desc = 生成一個傳送門
command-region-desc = 向您的區域內所有人發送訊息
command-reload_chunks-desc = 重新加載伺服器上的區塊
command-remove_lights-desc = 移除所有玩家生成的燈光
command-repair_equipment-desc = 修復所有已裝備的物品
command-reset_recipes-desc = 重置您的配方書
command-respawn-desc = 傳送到您的路徑點
command-revoke_build-desc = 撤銷玩家的建築區域權限
command-revoke_build_all-desc = 撤銷玩家的所有建築區域權限
command-safezone-desc = 創建一個安全區域
command-say-desc = 向所有聽得到的人發送訊息
command-scale-desc = 調整您的角色大小
command-server_physics-desc = 設置/取消帳戶的伺服器物理授權
command-set_motd-desc = 設置伺服器描述
command-ship-desc = 生成一艘船
command-site-desc = 傳送到某個場地
command-skill_point-desc = 為特定技能樹獲得技能點數
command-skill_preset-desc = 為您的角色設置所需技能
command-spawn-desc = 生成一個測試實體
command-sudo-desc = 以另一個實體的身份執行命令
command-tell-desc = 向另一個玩家發送訊息
command-tether-desc = 將另一個實體繫在您身上
command-time-desc = 設置一天中的時間
command-time_scale-desc = 設置時間的縮放比例
command-tp-desc = 傳送到另一個實體
command-rtsim_chunk-desc = 顯示當前區塊的 rtsim 信息
command-rtsim_info-desc = 顯示 rtsim NPC 的信息
command-rtsim_npc-desc = 按距離列出符合給定查詢的 rtsim NPC（例如：模擬商人）
command-rtsim_purge-desc = 在下次啟動時清除 rtsim 數據
command-rtsim_tp-desc = 傳送到 rtsim npc
command-unban-desc = 解除對給定使用者名稱的封禁
command-version-desc = 顯示伺服器版本
command-waypoint-desc = 設置當前位置為路徑點
command-weather_zone-desc = 創建一個天氣區域
command-whitelist-desc = 添加/移除白名單上的使用者名稱
command-wiring-desc = 創建連線元件
command-world-desc = 向伺服器上的所有人發送訊息

## Voxygen 客戶端指令

command-clear-desc = 清除聊天中的所有消息，影響所有聊天標籤
command-experimental_shader-desc = 切換實驗性著色器
command-help-desc = 顯示有關指令的信息
command-mute-desc = 靜音某玩家的聊天消息
command-unmute-desc = 取消對使用靜音指令靜音的玩家

# 結果與警告

command-no-permission = 您沒有權限使用 '/{ $command_name }'
command-position-unavailable = 無法獲取 { $target } 的位置
command-player-role-unavailable = 無法獲取 { $target } 的管理員角色
command-uid-unavailable = 無法獲取 { $target } 的 UID
command-area-not-found = 找不到名為 '{ $area }' 的區域
command-player-not-found = 找不到玩家 '{ $player }'！
command-player-uuid-not-found = 找不到 UUID 為 '{ $uuid }' 的玩家！
command-username-uuid-unavailable = 無法為使用者名稱 { $username } 確定 UUID
command-uuid-username-unavailable = 無法為 UUID  { $uuid } 確定使用者名稱
command-no-sudo = 冒充他人是不禮貌的
command-entity-dead = 實體 '{ $entity }' 已死亡！
command-error-write-settings = 寫入設置檔至磁碟失敗，但已成功寫入記憶體
  錯誤（存儲）：{ $error }
  成功（記憶體）：{ $message }
command-error-while-evaluating-request = 驗證請求時遇到錯誤：{ $error }
command-give-inventory-full = 玩家物品欄已滿，僅給予 { $given ->
  [1] 一件
  *[other] { $given } 件
}，總共 { $total } 件物品
command-give-inventory-success = 將 { $total } 件 { $item } 添加到物品欄
command-invalid-item = 無效的物品：{ $item }
command-invalid-block-kind = 無效的方塊類型：{ $kind }
command-nof-entities-at-least = 實體數應至少為 1
command-nof-entities-less-than = 實體數應小於 50
command-entity-load-failed = 加載實體配置失敗：{ $config }
command-spawned-entities-config = 從配置 { $config } 生成了 { $n } 個實體
command-invalid-sprite = 無效的精靈類型：{ $kind }
command-time-parse-too-large = { $n } 無效，不能超過 16 位數字
command-time-parse-negative = { $n } 無效，不能為負數
command-time-backwards = { $t } 在當前時間之前，時間不能倒退
command-time-invalid = { $t } 不是有效的時間
command-time-current = 現在是 { $t }
command-time-unknown = 時間未知
command-rtsim-purge-perms = 您必須是真正的管理員（而不只是臨時管理員）才能清除 rtsim 數據
command-chunk-not-loaded = 區塊 { $x }, { $y } 未加載
command-chunk-out-of-bounds = 區塊 { $x }, { $y } 不在地圖範圍內
command-spawned-entity = 生成的實體 ID 為：{ $id }
command-spawned-dummy = 生成了一個訓練假人
command-spawned-airship = 生成了一個飛空艇
command-spawned-campfire = 生成了一個營火
command-spawned-safezone = 生成了一個安全區域
command-volume-size-incorrect = 大小必須在 1 到 127 之間
command-volume-created = 創建了一個體積
command-permit-build-given = 您現在被允許在 '{ $area }' 建造
command-permit-build-granted = 授予了在 '{ $area }' 建造的權限
command-revoke-build-recv = 您在 '{ $area }' 的建築權限已被撤銷
command-revoke-build = 撤銷了 '{ $area }' 的建築權限
command-revoke-build-all = 您的所有建築權限已被撤銷
command-revoked-all-build = 已撤銷所有建築權限
command-no-buid-perms = 您無權建造
command-set-build-mode-off = 關閉了建築模式
command-set-build-mode-on-persistent = 開啟了建築模式，實驗性地形持久化已啟用，伺服器將嘗試保存更改，但無法保證
command-set-build-mode-on-unpersistent = 開啟了建築模式，更改將在區塊卸載時不再持久化
command-set_motd-message-added = 伺服器每日訊息設置為 { $message }
command-set_motd-message-removed = 移除了伺服器每日訊息
command-set_motd-message-not-set = 此本地化中沒有設置 motd
command-invalid-alignment = 無效的對齊方式：{ $alignment }
command-kit-not-enough-slots = 物品欄沒有足夠的空位
command-lantern-unequiped = 請先裝備燈籠
command-lantern-adjusted-strength = 您調整了火焰強度
command-lantern-adjusted-strength-color = 您調整了火焰強度和顏色
command-explosion-power-too-high = 爆炸威力不能超過 { $power }
command-explosion-power-too-low = 爆炸威力必須超過 { $power }
# 注意：此處不要翻譯 "confirm"
command-disconnectall-confirm = 請再次執行該指令並添加第二個參數 "confirm" 以確認您確實想要斷開所有玩家的伺服器連線
command-invalid-skill-group = { $group } 不是一個有效的技能組！
command-unknown = 未知的指令
command-disabled-by-settings = 伺服器設置中禁用了該指令
command-battlemode-intown = 您需要在城鎮中才能更改戰鬥模式！
command-battlemode-cooldown = 冷卻期間，請 { $cooldown } 秒後重試
command-battlemode-available-modes = 可用模式：pvp, pve
command-battlemode-same = 嘗試設置相同的戰鬥模式
command-battlemode-updated = 新的戰鬥模式：{ $battlemode }
command-buff-unknown = 未知的增益效果：{ $buff }
command-buff-data = 增益參數 '{ $buff }' 需要附加數據
command-buff-body-unknown = 未知的身體規格：{ $spec }
command-skillpreset-load-error = 加載預設時出錯
command-skillpreset-broken = 技能預設損壞
command-skillpreset-missing = 預設不存在：{ $preset }
command-location-invalid = 地點名稱 '{ $location }' 無效，名稱只能包含小寫 ASCII 和底線
command-location-duplicate = 地點 '{ $location }' 已存在，請考慮先刪除
command-location-not-found = 地點 '{ $location }' 不存在
command-location-created = 已創建地點 '{ $location }'
command-location-deleted = 已刪除地點 '{ $location }'
command-locations-empty = 目前沒有任何地點
command-locations-list = 可用地點：{ $locations }
# 注意：不要翻譯這些天氣名稱
command-weather-valid-values = 有效的值是 'clear', 'rain', 'wind' 和 'storm'
command-scale-set = 設置比例為 { $scale }
command-repaired-items = 修復了所有已裝備的物品
command-message-group-missing = 您正在使用群組聊天，但您不屬於任何群組，請使用 /world 或 /region 來更改聊天模式
command-tell-request = { $sender } 想與您對話
command-tell-to-yourself = 您不能對自己使用 /tell
command-transform-invalid-presence = 無法在當前狀態下進行變形
command-aura-invalid-buff-parameters = 光環的增益參數無效
command-aura-spawn = 生成了一個附加在實體上的新光環
command-aura-spawn-new-entity = 生成了一個新光環
command-reloaded-chunks = 重新加載了 { $reloaded } 區塊
command-server-no-experimental-terrain-persistence = 伺服器編譯時未啟用地形持久化
command-experimental-terrain-persistence-disabled = 實驗性地形持久化已禁用
command-adminify-assign-higher-than-own = 不能賦予比您自己永久角色更高的臨時角色
command-adminify-reassign-to-above = 不能重新分配比您角色或更高的角色
command-adminify-cannot-find-player = 無法找到玩家實體！
command-adminify-already-has-role = 玩家已經擁有該角色！
command-adminify-already-has-no-role = 玩家已經沒有該角色！
command-adminify-role-downgraded = 玩家 { $player } 的角色降級為 { $role }
command-adminify-role-upgraded = 玩家 { $player } 的角色升級為 { $role }
command-adminify-removed-role = 玩家 { $player } 的角色 { $role } 被移除
command-ban-added = 已將 { $player } 添加到封禁名單，原因：{ $reason }
command-ban-already-added = { $player } 已在封禁名單上
command-faction-join = 請使用 /join_faction 加入派系
command-group-join = 請先創建一個群組
command-group_invite-invited-to-group = 已邀請 { $player } 加入群組
command-group_invite-invited-to-your-group = { $player } 已被邀請加入您的群組
command-into_npc-warning = 希望您不是在濫用這個！
command-kick-higher-role = 不能踢出角色比您高的玩家
command-respawn-no-waypoint = 沒有設置路徑點
command-site-not-found = 找不到場地
command-sudo-higher-role = 不能對角色比您高的玩家使用 sudo
command-sudo-no-permission-for-non-players = 您無權 sudo 非玩家
command-time_scale-current = 當前時間比例為 { $scale }
command-time_scale-changed = 設置時間比例為 { $scale }
command-unban-successful = { $player } 已成功解封
command-unban-already-unbanned = { $player } 已經被解封
command-version-current = 伺服器運行於 { $hash }[{ $date }]
command-whitelist-added = 已將 { $username } 添加到白名單
command-whitelist-already-added = { $username } 已在白名單中！
command-whitelist-removed = 已將 { $username } 從白名單中移除
command-whitelist-unlisted = { $username } 不在白名單中
command-whitelist-permission-denied = 沒有權限移除用戶：{ $username }
command-outcome-variant_expected = 預期的結果變體
command-outcome-expected_body_arg = 預期的身體參數
command-outcome-expected_entity_arg = 預期的實體參數
command-outcome-expected_skill_group_kind = 預期的有效 ron SkillGroupKind
command-outcome-expected_frontent_specifier = 預期的前端規範
command-outcome-expected_integer = 預期的整數
command-outcome-expected_sprite_kind = 預期的 SpriteKind
command-outcome-invalid_outcome = { $outcome } 不是有效的結果
command-death_effect-unknown = 未知的死亡效果 { $effect }

# 無法觸及/無法測試但為了一致性添加

command-player-info-unavailable = 無法獲取 { $target } 的玩家信息
command-unimplemented-spawn-special = 尚未實現特殊實體的生成
command-kit-inventory-unavailable = 無法獲取物品欄
command-inventory-cant-fit-item = 物品無法放入物品欄
# 當您不存在時，由 /disconnect_all 發出（？）
command-you-dont-exist = 您不存在，所以無法使用該命令
command-destroyed-tethers = 所有繫繩已被摧毀！您現在自由了
command-destroyed-no-tethers = 您沒有連接任何繫繩
command-dismounted = 已下馬
command-no-dismount = 您沒有騎乘或被騎乘
