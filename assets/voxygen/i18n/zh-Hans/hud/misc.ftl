hud-show_tips = 显示提示
hud-quests = 任务
hud-you_died = 你死了
hud-waypoint_saved = 标记已保存
hud-sp_arrow_txt = 技能点
hud-inventory_full = 包裹已满
hud-someone_else = 其他人
hud-another_group = 其他队伍
hud-owned_by_for_secs = 归属 { $name } 还有 { $secs } 秒
hud-press_key_to_show_debug_info_fmt = 按下 { $key } 显示调试信息
hud-press_key_to_toggle_keybindings_fmt = 按下 { $key } 切换快捷键绑定
hud-press_key_to_toggle_debug_info_fmt = 按下 { $key } 切换显示调试信息
hud-press_key_to_respawn = 按下 { $key } 在你上次访问的篝火处重生.
hud-tutorial_btn = 教程
hud-tutorial_click_here = 按下 [ { $key } ] 释放鼠标光标并单击此按钮!
hud-tutorial_elements = 制作
hud-temp_quest_headline = 旅行者你好!
hud-temp_quest_text =
    想要开始旅程的话,可以浏览这个村庄并收集一些物资. 

    祝你你在旅途中随心所欲!

    查看屏幕的右下角,找到各种内容,例如背包,制作和地图.

    制作菜单可制作盔甲,武器,食物等等!

    城镇上到处都是野生动物,是皮革碎片的重要来源,可以为你提供一些防护来抵御危险.

    只要你准备就绪,就可以尝试挑战地图上的标记点,来获得更好的装备!
hud-spell = 法术
hud-diary = 技能书
hud-free_look_indicator =
    { $toggle ->
        [0] 自由视角已激活。按 { $key } 键关闭。
       *[other] 自由视角已激活。松开 { $key } 键关闭。
    }
hud-camera_clamp_indicator = 启用锁定视角. 按下 { $key } 禁用.
hud-auto_walk_indicator = 启用自动 行走/滑翔
hud-zoom_lock_indicator-remind = 缩放锁定
hud-zoom_lock_indicator-enable = 摄像头缩放锁定
hud-zoom_lock_indicator-disable = 摄像头缩放解锁
hud-activate = 激活
hud-deactivate = 停用
hud-collect = 收集
hud-pick_up = 拿起
hud-open = 打开
hud-use = 使用
hud-unlock-requires = 需要 { $item }
hud-unlock-consumes = 使用 { $item } 打开
hud-mine = 采集
hud-mine-needs_pickaxe = 需要镐
hud-mine-needs_unhandled_case = 需要 ???
hud-talk = 交谈
hud-trade = 交易
hud-mount = 攀爬
hud-sit = 坐下
hud-steer = 操控
hud-portal = 传送
-server = 服务器
-client = 客户端
hud-init-stage-singleplayer = 启动单人游戏服务器...
hud-init-stage-server-db-migrations = { "[" }{ -server }]: 应用数据库迁移...
hud-init-stage-server-db-vacuum = { "[" }{ -server }]: 清理数据库...
hud-init-stage-server-worldsim-erosion = { "[" }{ -server }]: 蚀刻 { $percentage }%
hud-init-stage-server-worldciv-civcreate = { "[" }{ -server }]: 生成 { $generated } 了 { $total } 村庄
hud-init-stage-server-worldciv-site = { "[" }{ -server }]: 生成村庄...
hud-init-stage-server-economysim = { "[" }{ -server }]: 模化经济...
hud-init-stage-server-spotgen = { "[" }{ -server }]: 生成场所...
hud-init-stage-server-starting = { "[" }{ -server }]: 启动服务器...
hud-init-stage-multiplayer = 开启多人服务器
hud-init-stage-client-connection-establish = { "[" }{ -client }]: 正在建立连接...
hud-init-stage-client-request-server-version = { "[" }{ -client }]: 等待服务器版本...
hud-init-stage-client-authentication = { "[" }{ -client }]: 认证中...
hud-init-stage-client-load-init-data = { "[" }{ -client }]: 从服务器加载初始化数据...
hud-init-stage-client-starting-client = { "[" }{ -client }]: 准备客户端...
hud-init-stage-render-pipeline = 创建渲染管线 ({ $done }/{ $total })
hud-items_lost_dur = 你装备的物品损失了耐久度。
hud-items_will_lose_dur = 你装备的物品会损失耐久度。
hud-hardcore_char_deleted = 此硬核模式角色已被删除。
hud-hardcore_will_char_deleted = 此硬核模式角色将会被删除。
hud-press_key_to_give_up = 按 { $key } 放弃被救援并立即死亡。
hud-steal-requires = 使用 { $item } 偷窃
hud-steal-consumes = 使用 { $item } 进行偷窃
hud-waypoint_interact = 设置路径点
hud-rest = 休息
hud-init-stage-server-worldsim-erosion_time_left =
    .days =
        { $n ->
            [one] 剩余约 { $n } 天
           *[other] 剩余约 { $n } 天
        }
    .hours =
        { $n ->
            [one] 剩余约 { $n } 小时
           *[other] 剩余约 { $n } 小时
        }
    .minutes =
        { $n ->
            [one] 剩余约 { $n } 分钟
           *[other] 剩余约 { $n } 分钟
        }
    .seconds =
        { $n ->
            [one] 剩余约 { $n } 秒
           *[other] 剩余约 { $n } 秒
        }
hud-tutorial-disable = 不再显示教学提示
hud-press_key_to_return_to_char_menu = 按 { $key } 返回人物菜单。
hud-downed_recieving_help = 正在接受帮助。
hud-steal = 窃取
hud-dig = 挖掘
hud-mine-needs_shovel = 需要铲子
hud-help = 帮助
hud-pet = 摸摸
hud-follow = 跟随
hud-stay = 停留
hud-read = 阅读
