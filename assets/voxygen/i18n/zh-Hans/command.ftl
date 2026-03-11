command-adminify-desc = 临时授予玩家受限的管理员权限，或移除当前权限（若未授予）
command-alias-desc = 更改您的别名
command-area_add-desc = 新增一个建造区域
command-area_list-desc = 列出所有建造区域
command-area_remove-desc = 移除指定建造区域
command-aura-desc = 创建一个光环
command-body-desc = s将您的角色变成不同种族
command-set_body_type-desc = 设置性别：女性或男性。
command-set_body_type-not_found =
    这不是有效的体型。
    请尝试以下选项之一：
    { $options }
command-set_body_type-no_body = 无法设置体型，因为目标没有身体。
command-set_body_type-not_character = 仅能对在线玩家角色永久设置体型。
command-buff-desc = 给玩家施加增益效果
command-build-desc = 开关建筑模式
command-battlemode-desc =
    设置你的战斗模式为：
    + pvp（玩家对战）
    + pve（玩家对战环境）
    如不带参数调用，将显示当前战斗模式。
command-battlemode_force-desc = 直接更改战斗模式，无需验证
command-campfire-desc = 生成一个篝火
command-help-template = { $usage }{ $description }
command-help-list =
    { $client-commands }
    { $server-commands }

    此外，您可以使用以下快捷键:
    { $additional-shortcuts }
command-airship-desc = 生成一艘空中飞船
command-ban-desc = 根据给定的用户名，对玩家进行禁用操作，持续时间由参数指定(如果提供)。传递true以覆盖并修改现有禁令。
command-ban-ip-desc = 封禁拥有指定用户名的玩家，期限为指定时长(若已提供)。与常规封禁不同，此操作还会额外封禁与该用户关联的IP地址。传递true可覆盖选项，则可将现有封禁状态进行更改。
command-clear_persisted_terrain-desc = 清除附近已存在的地形
command-create_location-desc = 在当前位置创建一个定位
command-death_effect-dest = 为目标实体添加一个死亡时效果
command-debug_column-desc = 打印有关某列的一些调试信息
command-debug_ways-desc = 打印有关列的存储方式的调试信息
command-delete_location-desc = 删除定位
command-destroy_tethers-desc = 摧毁所有与你相连的束缚
command-disconnect_all_players-desc = 断开与服务器上连接的所有玩家
command-dismount-desc = 如果你在骑乘，请先下马，或者卸载骑在你身上的任何东西
command-dropall-desc = 把你所有的物品扔到地上
command-make_block-desc = 在你的位置生成一个具有颜色的方块
command-make_npc-desc =
    在你附近从配置中生成一个实体。
    使用 Tab 键获取示例或自动补全 。
command-dummy-desc = 生成一个训练假人
command-explosion-desc = 让地面爆炸
command-faction-desc = 向您的派系发送讯息
command-give_item-desc = 给自己一些物品，使用tab键获取示例或自动完成。
command-gizmos-desc = 管理小工具订阅。
command-gizmos_range-desc = 更改小工具订阅的范围。
command-goto-desc = 传送到某个位置
command-goto-rand = 传送到随机位置
command-group-desc = 向您的群组发送讯息
command-group_invite-desc = 邀请玩家加入群组
command-group_kick-desc = 从群组中移除玩家
command-group_leave-desc = 离开当前群组
command-group_promote-desc = 提升某玩家为群组领导者
command-health-desc = 设置您当前的生命值
command-into_npc-desc = 将自己转换为NPC，请谨慎使用!
command-join_faction-desc = 加入/离开指定的派系
command-jump-desc = 偏移您当前的位置
command-kick-desc = 踢出某个名称的玩家
command-kill-desc = 自杀
command-kill_npcs-desc = 杀死NPC
command-kit-desc = 将一组物品放入您的物品栏。
command-lantern-desc = 更改您的灯笼强度和颜色
command-light-desc = 生成具有光线的实体
command-lightning-desc = 在当前位置放出闪电
command-location-desc = 传送到某个地点
command-outcome-desc = 创建一个结果
command-permit_build-desc = 给予玩家在某范围内建造的权限
command-players-desc = 列出当前在线的玩家
command-portal-desc = 生成一个传送门
command-region-desc = 向您的区域内所有人发送讯息
command-reload_chunks-desc = 重新加载服务器上的区块
command-repair_equipment-desc = 修复所有以装备的物品
command-reset_recipes-desc = 重置您的配方书
command-respawn-desc = 传送到您的路径点
command-revoke_build-desc = 撤销玩家的建筑区域权限
command-revoke_build_all-desc = 撤销玩家所有区域的建筑权限
command-safezone-desc = 创建一个安全区域
command-say-desc = 向所有听的到的人发送讯息
command-scale-desc = 调整您的角色大小
command-server_physics-desc = 设置/取消账户的服务器物理授权
command-set_motd-desc = 设置服务器描述
command-tell-desc = 向另一个玩家发送讯息
command-tether-desc = 将另一个实体系在您身上
command-time-desc = 设置一天中的时间
command-time_scale-desc = 设置时间的缩放比例
command-make_sprite-desc = 在你的位置创建一个精灵。要定义精灵属性，请使用 RON 语法指定一个 StructureSprite。
command-make_volume-desc = 创建一个空间体积（实验性功能）
command-motd-desc = 查看服务器描述
command-mount-desc = 骑乘一个实体
command-object-desc = 生成一个物体
command-poise-desc = 设置你当前的姿态
command-remove_lights-desc = 移除所有由玩家生成的光源
command-set-waypoint-desc = 将你的航点设置为当前位置。
command-ship-desc = 生成一艘船
command-site-desc = 传送到一个地点
command-skill_point-desc = 为某个技能树分配技能点
command-skill_preset-desc = 赋予你的角色所需的技能。
command-spawn-desc = 生成一个测试实体
command-spot-desc = 查找并传送到最近的特定类型地点。
command-sudo-desc = 以另一个实体的身份运行命令
