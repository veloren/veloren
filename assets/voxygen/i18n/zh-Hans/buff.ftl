## Regeneration

buff-heal = 治愈
    .desc = 生命值随时间持续恢复。
    .stat =
        { $duration ->
            [1] 在 { $duration } 秒内恢复 { $str_total } 点生命值。
           *[other] 在 { $duration } 秒内恢复 { $str_total } 点生命值。
        }

## Potion

buff-potion = 药水
    .desc = 吨吨吨...

## Saturation

buff-saturation = 饱腹
    .desc = 吃饱喝足后,一定时间内生命值随缓慢恢复.

## Campfire


## Energy Regen

buff-energy_regen = 耐力值恢复
    .desc = 更快的耐力值恢复速度。
    .stat =
        { $duration ->
            [1] 在 { $duration } 秒内恢复 { $str_total } 点耐力值。
           *[other] 在 { $duration } 秒内恢复 { $str_total } 点耐力值。
        }

## Health Increase

buff-increase_max_health = 提升最大生命值
    .desc = 你的最大生命值得到提升。
    .stat =
        { $duration ->
            [1]
                将最大生命值
                提升 { $strength } 点。
                持续 { $duration } 秒。
           *[other]
                将最大生命值
                提升 { $strength } 点。
                持续 { $duration } 秒。
        }

## Energy Increase

buff-increase_max_energy = 提升最大耐力值
    .desc = 你的最大耐力值得到提升。
    .stat =
        { $duration ->
            [1]
                将最大耐力值
                提升 { $strength } 点。
                持续 { $duration } 秒。
           *[other]
                将最大耐力值
                提升 { $strength } 点。
                持续 { $duration } 秒。
        }

## Invulnerability

buff-invulnerability = 无敌
    .desc = 免疫任何形式的攻击伤害。
    .stat =
        { $duration ->
            [1]
                获得无敌状态。
                持续 { $duration } 秒。
           *[other]
                获得无敌状态。
                持续 { $duration } 秒。
        }

## Protection Ward

buff-protectingward = 守护领域
    .desc = 有股力量在守护着你,一定时间内防御得到显著提升.

## Frenzied

buff-frenzied = 狂暴
    .desc = 你激发了非同寻常的速度，可以忽略轻伤.

## Haste

buff-hastened = 加速
    .desc = 你的移动和攻击速度变得更快.

## Bleeding

buff-bleed = 流血
    .desc = 造成定期伤害.

## Curse

buff-cursed = 诅咒
    .desc = 你被诅咒了.

## Burning

buff-burn = 着火
    .desc = 你快被活活烧死了。

## Crippled

buff-crippled = 残废
    .desc = 你的双腿受了重伤，你的运动能力受到影响.

## Freeze

buff-frozen = 冻结
    .desc = 你的行动和攻击都变慢了.

## Wet

buff-wet = 潮湿
    .desc = 地面变得湿滑，让你很难停下来.

## Ensnared

buff-ensnared = 陷阱
    .desc = 你的腿受到束缚，阻碍了你的移动.

## Fortitude

buff-fortitude = 刚毅
    .desc = 你对于震慑的承受能力更强大，并且受到的伤害月多对其他目标的震慑威力也提升了.

## Parried

buff-parried = 被格挡
    .desc = 你被格挡了，你的恢复更加缓慢.

## Potion sickness

buff-potionsickness = 药水抗性
    .desc = 近期饮用过药水后，药水对你的正面效果会降低。
    .stat =
        { $duration ->
            [1]
                降低后续药水的
                正面效果 { $strength } %。
                持续 { $duration } 秒。
           *[other]
                降低后续药水的
                正面效果 { $strength } %。
                持续 { $duration } 秒。
        }

## Reckless

buff-reckless = 狂热
    .desc = 你的攻击更加强大, 但对于你的防御疏忽关照(防御降低).

## Polymorped

buff-polymorphed = 形态转换
    .desc = 你的身体转换了形态.

## Util

buff-mysterious = 神秘效果

## Util

buff-remove = 点击删除
buff-resting_heal = 休息治疗
    .desc = 休息时每秒恢复 { $rate } % 生命值。
buff-combo_generation = 连击积蓄
    .desc = 随时间持续积蓄连击值。
    .stat =
        { $duration ->
            [1] 在 { $duration } 秒内生成 { $str_total } 连击值。
           *[other] 在 { $duration } 秒内生成 { $str_total } 连击值。
        }
buff-frigid = 霜冻
    .desc = 冻结你的敌人.
buff-imminentcritical = 致命一击
    .desc = 强化你的下一次攻击。
buff-fury = 暴怒
    .desc = 暴怒之下，你会造成更多的连击。
buff-defiance = 蔑视
    .desc = 你可以承受更强的攻击，且受击会增加连击数，与此相对你会变慢。
buff-bloodfeast = 嗜血
    .desc = 你从流血的敌人那里摄取生命。
buff-agility = 敏捷
    .desc =
        你的移动速度加快，
        但你的攻击力和防御力会大幅下降。
    .stat =
        { $duration ->
            [1]
                移动速度提升 { $strength } %。
                但作为代价，你的攻击和防御会急剧下降。
                持续 { $duration } 秒。
           *[other]
                移动速度提升 { $strength } %。
                但作为代价，你的攻击和防御会急剧下降。
                持续 { $duration } 秒。
        }
buff-sunderer = 破甲者
    .desc = 你的攻击可以击穿敌人的防御，并为你恢复更多耐力。
buff-scornfultaunt = 蔑视嘲讽
    .desc = 你蔑视并嘲讽你的敌人，获得强化的韧性与耐力。然而，如果你死亡，击杀者也将获得强化。
buff-rooted = 禁锢
    .desc = 你被困在原地，无法移动。
buff-winded = 气竭
    .desc = 你感到呼吸困难，耐力恢复量和移动速度都大幅下降。
buff-concussion = 脑震荡
    .desc = 你的头部遭到重击，难以集中注意力，导致你无法使用一些较复杂的攻击招式。
buff-staggered = 失衡
    .desc = 你失去了平衡，更容易受到重击。
buff-tenacity = 坚韧
    .desc = 你不仅能无视更沉重的打击，这些攻击还会为你提供耐力。然而，你的行动速度也会变慢。
buff-resilience = 坚韧
    .desc = 刚刚承受过沉重打击的你，将能更好地抵御后续的行动受阻效果。
buff-owltalon = 猫头鹰之爪
    .desc = 趁目标尚未察觉你的存在，你的下一次攻击将更加精准并造成更高伤害。
buff-heavynock = 重箭上弦
    .desc = 为长弓搭上一支更沉重的箭矢，使你的下一次射击能够震慑目标。不过，这种重箭在远程距离上的动量会较低。
buff-heartseeker = 寻心者
    .desc = 你的下一支箭将如刺穿心脏般击中敌人，造成更严重的创伤并为你提供能量。
buff-eagleeye = 鹰眼
    .desc = 你能清晰地看穿目标的弱点，并拥有足以让每一支箭都精准命中这些区域的敏捷性。
buff-chilled = 寒冷
    .desc = 剧烈的严寒使你的动作变得迟缓，并让你更容易受到强力攻击的影响。
buff-ardenthunter = 炽热猎人
    .desc = 你的狂热使箭矢对特定目标更具杀伤力，且每当箭矢命中目标时，你的能量都会随之增长。
buff-ardenthunted = 炽热猎物
    .desc = 你已被一名狂热的弓箭手标记为目标。
buff-septicshot = 腐蚀箭
    .desc = 你的下一支箭将使目标遭受感染。如果目标已处于任何负面状态下，这一击将更加致命。
buff-poisoned = 中毒
    .desc = 生命随风而逝…
buff-lifesteal = 生命汲取
    .desc = 吸取敌人生命。
buff-salamanderaspect = 螭龙之貌
    .desc = 阻燃并快速穿越岩浆。
buff-berserk = 狂暴
    .desc = 你处于狂暴状态，攻击更快更强，且移速变快。与此相对，你的防御力降低。
buff-heatstroke = 中暑
    .desc = 你暴露在酷暑之中且中暑了，你的能量回复和移速剧烈降低，一股寒意传来。
