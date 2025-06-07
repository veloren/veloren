main-username = 유저네임
main-server = 서버
main-password = 비밀번호
main-connecting = 연결중
main-creating_world = 세계 만드는 중
main-tip = 팁:
main-unbound_key_tip = 키설정 없음
main-notice =
    벨로렌의 알파 버전에 오신 것을 환영합니다!

    게임을 즐기시기 전에 다음 사항들을 꼭 참고해 주세요:

    - 지금은 매우 초기 단계의 알파 버전입니다. 버그가 많고, 게임플레이가 매우 미완성 상태이며, 시스템과 기능들도 다듬어지지 않았거나 아예 없는 경우도 많습니다.

    - 건설적인 피드백이나 버그 제보가 있다면 GitLab 저장소 또는 Discord, Matrix 서버를 통해 저희에게 알려주세요.

    - 벨로렌은 오픈소스 프로젝트입니다. GNU 일반 공중 사용 허가서 버전 3에 따라 자유롭게 게임을 플레이, 수정, 재배포하실 수 있습니다.

    - 벨로렌은 비영리 커뮤니티 프로젝트이며, 참여하는 모든 사람들은 자원봉사자입니다.
    마음에 드신다면 언제든 저희 작업 그룹 중 하나에 참여하실 수 있습니다!

    안내문을 읽어 주셔서 감사합니다. 즐거운 플레이 되시길 바랍니다!

    ~ 개발팀
main-login_process =
    멀티플레이어 모드에 대하여:

    인증이 활성화된 서버에서 플레이하려면 계정이 필요합니다.

    계정을 생성하는 곳에 대하여:
    https://veloren.net/account/
main-login-server_not_found = 서버를 찾지 못했습니다.
main-login-authentication_error = 서버에서 인증 오류가 발생했습니다.
main-login-internal_error = 클라이언트에서 내부 오류가 발생했습니다. 힌트: 플레이어 캐릭터가 삭제되었을 수 있습니다.
main-login-failed_auth_server_url_invalid = 인증 서버에 연결하지 못했습니다.
main-login-insecure_auth_scheme = HTTP 인증 방식은 지원되지 않습니다. 이는 안전하지 않습니다! 개발 목적을 위해 'localhost'나 디버그 빌드에서는 HTTP가 허용됩니다.
main-login-server_full = 서버가 가득 찼습니다.
main-login-untrusted_auth_server = 인증 서버가 신뢰할 수 없습니다.
main-login-timeout = 시간 초과: 서버가 제시간에 응답하지 않았습니다. 힌트: 서버가 현재 과부하 상태이거나 네트워크에 문제가 있을 수 있습니다.
main-login-server_shut_down = 서버가 닫혔습니다.
main-login-network_error = 네트워크 오류.
main-login-network_wrong_version = 서버와 클라이언트 버전이 일치하지 않습니다. 힌트: 게임 클라이언트를 업데이트해야 할 수 있습니다.
main-login-failed_sending_request = 인증 서버에 대한 요청이 실패했습니다.
main-login-invalid_character = 선택한 캐릭터가 유효하지 않습니다.
main-login-client_crashed = 클라이언트 충돌.
main-login-not_on_whitelist = 접속하려는 서버의 화이트리스트에 회원으로 등록되어 있지 않습니다.
main-login-banned = 다음 이유로 영구적으로 차단되었습니다: { $reason }
main-login-kicked = 다음 이유로 추방 되었습니다: { $reason }
main-login-select_language = 언어 선택
main-login-client_version = 클라이언트 버전
main-login-server_version = 서버 버전
main-login-client_init_failed = 클라이언트 실행 실패: { $init_fail_reason }
main-login-username_bad_characters = 사용자 이름에 유효하지 않은 문자가 포함되어 있습니다! (영문자, 숫자, '_' 및 '-'만 허용됩니다).
main-login-username_too_long = 유저네임이 너무 깁니다! 최대 글자수는: { $max_len }
main-servers-select_server = 서버를 선택하세요
main-servers-singleplayer_error = 내부 서버에 연결 실패: { $sp_error }
main-servers-network_error = 서버 네트워크/소켓 오류: { $raw_error }
main-servers-participant_error = 연결 끊김/프로토콜 오류: { $raw_error }
main-servers-stream_error = 클라이언트 연결/압축/(역)직렬화 오류: { $raw_error }
main-servers-database_error = 서버 데이터베이스 오류: { $raw_error }
main-servers-persistence_error = 서버 지속성 오류 (에셋/캐릭터 데이터 관련 오류일 가능성): { $raw_error }
main-servers-other_error = 서버 일반 오류: { $raw_error }
main-credits = 크레딧
main-credits-created_by = 만든 이
main-credits-music = 음악
main-credits-fonts = 폰트
main-credits-other_art = 기타 아트
main-credits-contributors = 공헌자
loading-tips =
    .a0 = '{ $gameinput-togglelantern }' 키를 눌러 등불을 키세요.
    .a1 = '{ $gameinput-controls }' 키를 눌러 기본 키설정을 보세요.
    .a2 = /say 나 /s 를 쳐서 근처에 있는 플레이어들 하고만 대화를 할수 있습니다.
    .a3 = /region 나 /r 를 쳐서 몇백 블록 안에 있는 플레이어들 하고만 대화를 할수 있습니다.
    .a4 = 어드민은 /build 명령어로 건축 모드를 사용할수 있습니다.
    .a5 = /group 나 /g 를 쳐서 그룹 안에 있는 플레이어들 하고만 대화를 할수 있습니다.
    .a6 = 개인적인 대화를 나누려면 /tell 뒤에 플레이어의 유저네임과 전할 내용을 치면 됩니다.
    .a7 = 눈을 크게 뜨고 전 세계에 흩어져 있는 상자와 아이템들을 찾아보세요!
    .a8 = 가방이 음식으로 가득 찼나요? 그 음식들을 조합하여 더 좋은 음식을 만들어 보세요!
    .a9 = 무엇을 해야할지 잘 모르겠습니까? 지도에 있는 던전들에 가보세요!
    .a10 = 시스템에 맞게 그래픽 설정을 바꾸는 것을 잊지 마세요. '{ $gameinput-settings }' 키로 설정을 열수 있습니다.
    .a11 = 다른 사람들과 함께 하면 더 재밌습니다! '{ $gameinput-social }' 키를 눌러 누가 온라인 상태인지 볼수 있습니다.
    .a12 = '{ $gameinput-dance }' 키를 눌러 춤을 추세요. 야호!
    .a13 = '{ $gameinput-glide }' 키를 눌러 글라이더를 펼쳐 하늘을 마음껏 활공하세요.
    .a14 = 벨로렌은 아직 프리알파 입니다. 다들 매일 게임을 더 좋게 만들기 위해 노력하고 있어요!
    .a15 = 게임 개발에 참여하고 싶거나 개발자들과 대화하고 싶다면 디스코드 서버에 가입하세요.
    .a16 = 설정에서 체력이 어떻게 보이는지 바꿀수 있습니다.
    .a17 = '{ $gameinput-sit }' 키를 눌러 모닥불 근처에 앉아서 상처를 더 빨리 낫게 하세요.
    .a18 = 모험하는데 가방칸이 모자라거나 더 좋은 옷이 필요한가요? '{ $gameinput-crafting }' 키를 눌러 제작 메뉴를 여세요!
    .a19 = '{ $gameinput-roll }' 키를 눌러 구르세요. 구르면 적의 공격을 피하거나 더 빨리 움직일수 있습니다.
    .a20 = 어느 물건이 어디에 쓰이는지 궁금하세요? 'input:<item name>'를 제작 메뉴에서 검색하여 어디에 쓰이는지 알아보세요.
    .a21 = 멋진 것을 발견했나요? '{ $gameinput-screenshot }' 키로 스크린샷을 찍을 수 있습니다.
main-singleplayer-seed = 시드
main-singleplayer-delete = 삭제
main-singleplayer-random_seed = 무작위
main-singleplayer-create_custom = 커스텀 생성
main-singleplayer-new = 신규
main-singleplayer-regenerate = 재생성
main-singleplayer-size_lg = 로그 크기
main-singleplayer-map_large_warning = 경고: 대규모 월드는 초기 시작에 시간이 오래 걸릴 수 있습니다.
main-singleplayer-map_erosion_quality = 침식도
main-singleplayer-world_name = 월드 이름
main-singleplayer-play = 플레이
menu-singleplayer-confirm_delete = { $world_name }을 정말 삭제하시겠습니까?
menu-singleplayer-confirm_regenerate = { $world_name }을(를) 다시 생성하시겠습니까?
main-singleplayer-map_shape-square = 사각형
main-singleplayer-map_scale = 수직 스케일
main-singleplayer-map_shape = 형태
main-singleplayer-day_length = 하루 지속 시간
main-server-rules = 이 서버에는 반드시 동의해야 하는 규칙이 있습니다.
main-singleplayer-generate_and_play = 생성 및 플레이
main-singleplayer-map_shape-circle = 원형
main-login-banned_until =
    다음 이유로 임시 차단되었습니다: { $reason }
    차단 해제일: { $end_date }
main-singleplayer-map_large_extra_warning = 이것은 기본 옵션으로 { $count }개의 세계를 생성하는 데 드는 자원과 거의 같은 양의 자원을 소모할 것입니다.
main-server-rules-seen-before = 이 규칙은 마지막으로 동의한 이후 변경되었습니다.
