## Player events
hud-chat-online_msg = [{ $name }]이(가) 현재 온라인
hud-chat-offline_msg = [{ $name }]이(가) 현재 오프라인
## Buff outcomes
hud-outcome-burning = 사인: 화상
hud-outcome-curse = 사인: 저주
hud-outcome-bleeding = 사인: 출혈
hud-outcome-crippled = 사인: 다리 부러짐
hud-outcome-frozen =사인: 동사
hud-outcome-mysterious = 사인: 비밀
## Buff deaths
hud-chat-died_of_pvp_buff_msg = [{ $attacker }]이(가) [{ $victim }]을(를) 죽임. { $died_of_buff } 
hud-chat-died_of_buff_nonexistent_msg = [{ $victim }] { $died_of_buff }
hud-chat-died_of_npc_buff_msg = { $attacker }이(가) [{ $victim }]을(를) 죽임. { $died_of_buff } 
## PvP deaths
hud-chat-pvp_melee_kill_msg = [{ $attacker }]이(가) [{ $victim }]을(를) 쓰러트림
hud-chat-pvp_ranged_kill_msg = [{ $attacker }]이(가) [{ $victim }]을(를) 쏴죽임
hud-chat-pvp_explosion_kill_msg = [{ $attacker }]이(가) [{ $victim }]을(를) 터트려 죽임
hud-chat-pvp_energy_kill_msg = [{ $attacker }]이(가) [{ $victim }]을(를) 마법으로 죽임
hud-chat-pvp_other_kill_msg = [{ $attacker }]이(가) [{ $victim }]을(를) 죽임
## PvE deaths
hud-chat-npc_melee_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 죽임
hud-chat-npc_ranged_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 쏴죽임
hud-chat-npc_explosion_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 터트려 죽임
hud-chat-npc_energy_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 마법으로 죽임
hud-chat-npc_other_kill_msg = { $attacker }이(가) [{ $victim }]을(를) 죽임
## Other deaths
hud-chat-environmental_kill_msg = [{ $name }]이(가) { $environment }에서 죽음
hud-chat-fall_kill_msg = [{ $name }]이(가) 떨어져 죽음
hud-chat-suicide_msg = [{ $name }]이(가) 자해로 인해 사망
hud-chat-default_death_msg = [{ $name }]이(가) 죽음
## Utils
hud-chat-all = 모두
hud-chat-you = 당신
hud-chat-mod = 모드
hud-chat-chat_tab_hover_tooltip = 오른쪽 마우스 클릭으로 설정
hud-loot-pickup-msg = {$actor}이(가) { $amount ->
   [one] { $item }
   *[other] {$amount}x {$item} 주음
}
hud-chat-loot_fail = 가방이 가득 찼습니다!
hud-chat-goodbye = 안녕!
hud-chat-connection_lost = 연결 끊김. { $time }초후 킥.