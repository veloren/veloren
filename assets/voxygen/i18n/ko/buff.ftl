## Regeneration

buff-heal = 회복
    .desc = 일정 시간마다 체력을 회복한다.
    .stat =
        { $duration ->
            [1] { $duration } 초 동안 { $str_total } 체력을 회복한다.
           *[other] { $duration } 초 동안 { $str_total } 체력을 회복한다.
        }

## Potion

buff-potion = 물약
    .desc = 마시는 중...

## Saturation

buff-saturation = 포만감
    .desc = 음식을 먹어 일정 시간마다 체력을 회복한다.

## Campfire


## Energy Regen

buff-energy_regen = 에너지 재생
    .desc = 더 빠른 에너지 재생.
    .stat =
        { $duration ->
            [1] { $duration }초 동안 { $str_total }의 에너지를 회복합니다.
           *[other] { $duration }초 동안 { $str_total }의 에너지를 회복합니다.
        }

## Health Increase

buff-increase_max_health = 최대 체력 증가
    .desc = 최대 HP가 증가합니다.
    .stat =
        { $duration ->
            [1]
                최대 체력 증가.
                { $strength } 만큼.
                { $duration }초 동안 지속됩니다.
           *[other]
                최대 체력 증가.
                { $strength } 만큼.
                { $duration }초 동안 지속됩니다.
        }

## Energy Increase

buff-increase_max_energy = 최대 기력 증가
    .desc = 최대 기력이 증가합니다.
    .stat =
        { $duration ->
            [1]
                최대 기력 증가
                { $strength } 만큼.
                { $duration }초 동안 지속됩니다.
           *[other]
                최대 기력 증가
                { $strength } 만큼.
                { $duration }초 동안 지속됩니다.
        }

## Invulnerability

buff-invulnerability = 무적
    .desc = 어떤 공격으로도 피해를 입지 않습니다.
    .stat =
        { $duration ->
            [1]
                무적 부여
                { $duration }초 동안 지속됩니다.
           *[other]
                무적 부여
                { $duration }초 동안 지속됩니다.
        }

## Protection Ward

buff-protectingward = Protecting Ward
    .desc = You are protected, somewhat, from attacks.

## Frenzied

buff-frenzied = 광포화
    .desc = 속도가 빨라지고 작은 상처를 무시할 수 있다.

## Haste

buff-hastened = 신속
    .desc = 이동 속도와 공격 속도가 빨라진다.

## Bleeding

buff-bleed = 출혈
    .desc = 피해를 입는다.

## Curse

buff-cursed = 저주
    .desc = 저주를 받았다.

## Burning

buff-burn = 불붙음
    .desc = 산채로 불타고 있다.

## Crippled

buff-crippled = 다리 부러짐
    .desc = 다리가 심하게 다쳐 이동 속도가 느려졌다.

## Freeze

buff-frozen = 얼음
    .desc = 이동 속도와 공격 속도가 느려졌다.

## Wet

buff-wet = 젖음
    .desc = 발이 미끄러워 멈추기가 어려워졌다.

## Ensnared

buff-ensnared = 발묶임
    .desc = 덩쿨이 다리를 휘감고 있어 움직일 수가 없다.

## Fortitude

buff-fortitude = 불굴
    .desc = 경직되지 않는다.

## Parried

buff-parried = 받아넘겨짐
    .desc = 상대가 공격을 받아넘겨서 자세가 무너졌다.

## Util

buff-remove = 클릭하여 제거
buff-agility = 민첩성
    .desc =
        이동 속도가 빨라지지만,
        공격력과 방어력이 감소하고 받는 피해가 늘어난다.
    .stat =
        { $duration ->
            [1]
                이동 속도가 { $strength } % 증가한다.
                그 대신 공격력과 방어력이 크게 감소한다.
                지속 시간 { $duration } 초.
           *[other]
                이동 속도가 { $strength } % 증가한다.
                그 대신 공격력과 방어력이 크게 감소한다.
                지속 시간 { $duration } 초.
        }
buff-poisoned = 중독
    .desc = 생명이 서서히 시들어가는 느낌이 듭니다...
buff-scornfultaunt = 조롱의 도발
    .desc = 적들을 조롱하며 도발하여, 자신에게 강화된 체력과 스태미나를 부여합니다. 하지만, 당신의 죽음은 당신을 처치한 자를 강화시킵니다.
buff-rooted = 뿌리 박힘
    .desc = 당신은 제자리에 고정되어 움직일 수 없습니다.
buff-mysterious = 불가사의한 효과
buff-winded = 호흡곤란
    .desc = 거의 숨을 쉴 수 없어 에너지를 회복하는 속도와 이동 속도가 저하됩니다.
buff-concussion = 뇌진탕
    .desc = 머리를 심하게 맞아 집중하는 데 어려움을 겪으며, 일부 복잡한 공격을 사용할 수 없습니다.
buff-staggered = 휘청거림
    .desc = 균형을 잃고 무거운 공격에 더 취약해졌습니다.
buff-tenacity = 고집
    .desc = 더 강한 공격을 견디는 것뿐만 아니라, 그런 공격이 오히려 당신에게 힘을 줍니다. 하지만 당신은 더 느려집니다.
buff-potionsickness = 포션 중독
    .desc = 최근에 포션을 소비한 후, 포션의 효과가 감소합니다.
    .stat =
        { $duration ->
            [1]
                긍정적인 효과 감소
                포션의 효과가 { $strength }% 감소합니다.
                { $duration }초 동안 지속됩니다.
           *[other]
                긍정적인 효과 감소
                포션의 효과가 { $strength }% 감소합니다.
                { $duration }초 동안 지속됩니다.
        }
buff-resting_heal = 휴식 회복
    .desc = 휴식 중 { $rate } %의 HP가 초당 회복된다.
buff-resilience = 회복탄력성
    .desc = 치명적인 공격을 받은 후, 이후의 무력화 효과에 더 강해집니다.
buff-reckless = 무모
    .desc = 당신의 공격력이 강해지지만, 방어가 허술해집니다.
buff-polymorphed = 변화
    .desc = 당신의 몸이 다른 형태로 변화합니다.
buff-frigid = 냉혹
    .desc = 적들을 얼려버립니다.
buff-lifesteal = 생명 흡수
    .desc = 적의 생명을 흡수합니다.
buff-heatstroke = 열사병
    .desc = 열에 과하게 노출되어 열사병에 시달리고 있습니다. 에너지 회복과 이동 속도가 감소합니다. 열 좀 식히세요.
buff-salamanderaspect = 샐러맨더 형태
    .desc = 불에 타지 않고 용암에서 빠르게 헤엄칩니다.
buff-imminentcritical = 임박한 치명타
    .desc = 다음 공격이 적에게 치명타를 가합니다.
buff-fury = 분노
    .desc = 분노를 통해 당신의 공격이 더 많은 콤보를 생성합니다.
buff-sunderer = 절단자
    .desc = 당신의 공격이 상대의 방어력을 관통하고 더 많은 에너지를 회복합니다.
buff-defiance = 저항
    .desc = 더 강하게 뒤흔드는 공격을 버텨낼 수 있게 되지만, 더 느려집니다.
buff-bloodfeast = 피의 축제
    .desc = 피를 흘리는 적을 공격하면 생명력을 회복합니다.
buff-berserk = 광전사
    .desc = 분노가 타오릅니다. 공격이 더욱 강하고 빨라지며 이동 속도가 상승하지만, 대가로 방어 능력이 하락합니다.
