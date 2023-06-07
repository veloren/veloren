main-username = 유저네임
main-server = 서버
main-password = 비밀번호
main-connecting = 연결중
main-creating_world = 세계 만드는 중
main-tip = 팁:
main-unbound_key_tip = 키설정 없음
main-notice =
    벨로렌의 알파 버전에 오신것을 환영합니다!
    
    게임을 즐기기 전에 다음 내용을 상기해 주세요:
    
    - 이 버전은 매우 이른 알파 버전입니다. 버그도 많고 기능도 없는게 많으며 있어도 아직은 미완성인채 입니다.
    
    - 건의할것이 있거나 버그가 있다면 레딧이나 깃랩, 디스코드를 통해 개발자들에게 연락할 수 있습니다.
    
    - 벨로렌은 GPL v3 규정에 따라 만들어졌습니다.
    그 말은 당신이 GNU 일반 공중 사용 허가서 3판 규정에 따라 자유롭게 이 게임을 플레이하고 개작하거나 재배포 할수 있다는 뜻입니다.
    
    - 벨로렌은 비영리로 모두가 자진하여 무보수로 만들어가는 게임입니다.
    게임이 마음에 들어 돕고 싶다면 개발이나 아트팀에 참여해도 됩니다!
    
    이 글을 읽어주셔 감사하고 게임을 재미있게 즐겨주세요!
    
    ~ 벨로렌 개발자들
main-login_process =
    로그인 정보:
    
    인증 기능이 켜진 서버에 접속하려면
    계정을 만들어야 합니다.
    
    계정은 다음 링크를 통해 만들수 있습니다.
    
    https://veloren.net/account/.
main-login-server_not_found = 서버를 찾지 못함
main-login-authentication_error = 서버에서 인증 오류
main-login-internal_error = 클라이언트 내부 오류 (플레이어의 캐릭터가 삭제되었을 가능성이 높습니다)
main-login-failed_auth_server_url_invalid = 인증 서버에 연결 실패
main-login-insecure_auth_scheme = HTTP 인증 방식은 지원되지 않습니다. 개발 과정을 위해 HTTP는 'localhost'나 디버그 빌드 용으로만 있습니다.
main-login-server_full = 서버가 가득 참
main-login-untrusted_auth_server = 인증 서버가 신뢰되지 않음
main-login-outdated_client_or_server = 서버가 화남: 버전이 맞지 않을 가능성이 높음. 업데이트를 해주세요.
main-login-timeout = 타임아웃: 서버가 시간 내에 응답하지 않음. (서버 과부하나 인터넷 연결 문제).
main-login-server_shut_down = 서버가 문닫음
main-login-network_error = 네트워크 오류
main-login-network_wrong_version = 서버와 클라이언트 버전이 맞지 않음. 게임을 업데이트 해주세요.
main-login-failed_sending_request = 인증 서버에 요청 실패
main-login-invalid_character = 선택한 캐릭터를 사용하지 못함
main-login-client_crashed = 클라이언트 충돌
main-login-not_on_whitelist = 화이트리스트에 등록되여야 연결 가능
main-login-banned = 다음 이유로 서버에서 밴당했습니다.
main-login-kicked = 다음 이유로 서버에서 킥되었습니다.
main-login-select_language = 언어 선택
main-login-client_version = 클라이언트 버전
main-login-server_version = 서버 버전
main-login-client_init_failed = 클라이언트 실행 실패: { $init_fail_reason }
main-login-username_bad_characters = 유저네임에 사용하지 못하는 글자가 있습니다! (영문, 숫자, '_'과 '-'만 사용 가능)
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
    .a1 = '{ $gameinput-help }' 키를 눌러 기본 키설정을 보세요.
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