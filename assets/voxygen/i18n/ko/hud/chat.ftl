## Player events

hud-chat-online_msg = { "[" }{ $name }]이(가) 현재 온라인
hud-chat-offline_msg = { "[" }{ $name }]이(가) 현재 오프라인

## Buff deaths

hud-chat-died_of_pvp_buff_msg =
    .burning = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 죽임. 사인: 화상
    .bleeding = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 죽임. 사인: 출혈
    .curse = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 죽임. 사인: 저주
    .crippled = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 죽임. 사인: 다리 부러짐
    .frozen = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 죽임. 사인: 동사
    .mysterious = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 죽임. 사인: 비밀
hud-chat-died_of_buff_nonexistent_msg =
    .burning = { "[" }{ $victim }] 사인: 화상
    .bleeding = { "[" }{ $victim }] 사인: 출혈
    .curse = { "[" }{ $victim }] 사인: 저주
    .crippled = { "[" }{ $victim }] 사인: 다리 부러짐
    .frozen = { "[" }{ $victim }] 사인: 동사
    .mysterious = { "[" }{ $victim }] 사인: 비밀
hud-chat-died_of_npc_buff_msg =
    .burning = { $attacker }이(가) [{ $victim }]을(를) 죽임. 사인: 화상
    .bleeding = { $attacker }이(가) [{ $victim }]을(를) 죽임. 사인: 출혈
    .curse = { $attacker }이(가) [{ $victim }]을(를) 죽임. 사인: 저주
    .crippled = { $attacker }이(가) [{ $victim }]을(를) 죽임. 사인: 다리 부러짐
    .frozen = { $attacker }이(가) [{ $victim }]을(를) 죽임. 사인: 동사
    .mysterious = { $attacker }이(가) [{ $victim }]을(를) 죽임. 사인: 비밀

## PvP deaths

hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 쓰러트림
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 쏴죽임
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 터트려 죽임
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 마법으로 죽임
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }]이(가) [{ $victim }]을(를) 죽임

## PvE deaths

hud-chat-npc_melee_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 죽임
hud-chat-npc_ranged_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 쏴죽임
hud-chat-npc_explosion_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 터트려 죽임
hud-chat-npc_energy_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 마법으로 죽임
hud-chat-npc_other_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 죽임

## Other deaths

hud-chat-fall_kill_msg = { "[" }{ $name }]이(가) 떨어져 죽음
hud-chat-suicide_msg = { "[" }{ $name }]이(가) 자해로 인해 사망
hud-chat-default_death_msg = { "[" }{ $name }]이(가) 죽음

## Utils

hud-chat-all = 모두
hud-chat-chat_tab_hover_tooltip = 오른쪽 마우스 클릭으로 설정
hud-loot-pickup-msg =
    { $actor }이(가) { $amount ->
        [one] { $item }
       *[other] { $amount }x { $item } 주음
    }
hud-chat-goodbye = 안녕!
hud-chat-connection_lost = 연결 끊김. { $time }초후 킥.
