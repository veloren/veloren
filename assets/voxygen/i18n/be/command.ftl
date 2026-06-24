command-help-template = { $usage } { $description }
command-help-list =
    { $client-commands }
    { $server-commands }

    Апроч таго, вы можаце скарыстаць наступныя спалучэнні клавіш:
    { $additional-shortcuts }
command-adminify-desc = Часова падае гульцу абмежаваную ролю адміністратара ці падаляе бягучую (калі яна не пададзена)
command-airship-desc = Стварае дырыжабль
command-alias-desc = Змяніце свой псеўданім
command-area_add-desc = Дадае новую зону будавання
command-area_list-desc = Паказвае ўсе зоны будавання
command-area_remove-desc = Прыбірае пэўную зону будавання
command-aura-desc = Стварае аўру
command-body-desc = Змяняе ваш выгляд цела на іншы
command-set_body_type-desc = Аберыце тып целаскладу – Мужчынскі ці Жаночы.
command-set_body_type-not_found =
    Недазволены тып фігуры.
    Паспрабуйце адзін з:
    { $options }
command-set_body_type-no_body = Не атрымалася ўсталяваць тып цела, бо ў аб'екта няма цела.
command-set_body_type-not_character = Тып целаскладу можа быць усталяваны толькі ў тым выпадку, калі персанаж гульца ў сетке.
command-buff-desc = Ужывае ўзмацненне на гульца
command-build-desc = Уключае і выключае рэжым будавання
command-ban-desc = Забаніць гульца з дадзеным імем на пэўны тэрмін (калі ўказан). Перадайце true для перазапісу, каб змяніць існы бан.
command-ban-ip-desc = Забаніць гульца з дадзеным імем на пэўны тэрмін (калі паказан). У адрозненне ад звычайнага бана, пры гэтым таксама дадатковае блакуецца IP-адрас, злучаны з гэтым карыстачом. Перадайце true для перазапісу, каб змяніць існы бан.
command-battlemode-desc =
    Усталюйце рэжым бою на:
    + pvp (гулец супраць гульца)
    + pve (гулец супраць атачэння).
    Пры выкліку без аргументаў будзе паказаны бягучы рэжым бою.
command-battlemode_force-desc = Змяняйце свой сцяг баявога рэжыму без якіх-небудзь праверак
command-campfire-desc = Стварае вогнішча
command-clear_persisted_terrain-desc = Чысціць блізкую захаваную мясцовасць
command-create_location-desc = Стварае месціва на бягучай пазіцыі
command-death_effect-dest = Дадае аб'екту эфект "На мяжы смерці"
command-debug_column-desc = Выводзіць дэбаг-інфармацыю пра калонку
command-debug_ways-desc = Выводзіць дэбаг-інфармацыю пра спосабы калонкі
command-delete_location-desc = Выдаліць месціва
command-destroy_tethers-desc = Знішчыце ўсе прывязкі, злучаныя з вамі
command-disconnect_all_players-desc = Адключае ўсіх гульцоў ад сервера
command-dismount-desc = Вы спешвайцеся, калі едзеце верхам, ці здымаеце тое, што едзе на вас
command-dropall-desc = Выкідвае ўсе вашы прадметы на зямлю
command-dummy-desc = Стварае трэнеравальны манекен
command-explosion-desc = Узрывае зямлю вакол вас
command-faction-desc = Адпраўляе паведамленні вашай фракцыі
command-give_item-desc = Выдае вам некаторыя прадметы. Для прыкладу ці для аўтазапаўнення скарыстайце Tab.
command-gizmos-desc = Кіраванне падпіскамі на гаджэты.
command-gizmos_range-desc = Змяніць дыяпазон падпісак на гаджэт.
command-goto-desc = Тэлепартацыя ў патрэбнае месца
command-goto-rand = Тэлепартацыя на выпадковую пазіцыю
command-group-desc = Адпраўленне паведамленняў вашай групе
command-group_invite-desc = Запрасіць гульца ў групу
command-group_kick-desc = Выгнаць гульца з групы
command-group_leave-desc = Пакінуць бягучую групу
command-group_promote-desc = Перадаць гульцу права лідара групы
command-health-desc = Усталяваць ваша бягучае здароўя
command-into_npc-desc = Ператварыцца ў НПС. Будзьце асцярожныя!
command-join_faction-desc = Далучыцца/пакінуць вызначаную фракцыю
command-jump-desc = Зрушыць бягучае становішча
command-kick-desc = Выштырыць гульца з паказаным імём карыстача
command-kill-desc = Забіць сябе
command-kill_npcs-desc = Забіць NPC
command-kit-desc = Змясціць набор прадметаў у свой інвентар.
command-lantern-desc = Змяніць моц і колер вашага ліхтара
command-light-desc = Стварыць сутнасць з святлом
command-lightning-desc = Стварыць удар маланкі на бягучаю пазіцыю
command-location-desc = Тэлепартацыя да месца
command-make_block-desc = Зрабіць каляровы блок на вашым месцазнаходжанні
command-make_npc-desc =
    Стварыць сутнасць з канфіга побач з вамі.
    Для прыкладу або для аўтазавяршэння націсніце Tab.
command-make_sprite-desc = Стварыць спрайт на вашым месцы, каб вызначыць атрыбуты спрайта, выкарыстоўвайце ron syntax для StructureSprite.
command-make_volume-desc = Стварыць volume (эксперыментальны)
command-motd-desc = Праглядзець апісанне сервера
command-mount-desc = Зманціраваць іста
command-object-desc = Стварыць аб'ект
command-outcome-desc = Стварыць вынік
command-players-desc = Паказаць гульцоў, якія зараз анлайн
command-poise-desc = Указаць ваша бягучае становішча
command-portal-desc = Стварыць Партал
command-region-desc = Даслаць паведамленне ўсім у вашым рэгіёне свету
command-reload_chunks-desc = Абнавіць чанкі, загружаныя на сервер
command-remove_lights-desc = Выдаліць усе ліхтары, згенераваныя гульцамі
command-repair_equipment-desc = Адрамантаваць усе абсталяваныя прадметы
command-reset_recipes-desc = Скінуць вашу кнігу рэцэптаў
command-respawn-desc = Тэлепартавацца да вашай пазнакі
command-revoke_build-desc = Адмяніць дазвол гульцу на будаўніцтва ў зоне будаўніцтва
command-revoke_build_all-desc = Адмяніць усе дазволы гульцу ў будаўнічай зоне
command-safezone-desc = Стварыць бяспечную зону
command-permit_build-desc = Падаць гульцу абмежаваную прастору, ў якой ён можа будаваць
command-say-desc = Адправіць паведамленні ўсім, хто знаходзіцца на адлегласці голасу
command-scale-desc = Наладзіць памер свайго персанажа
command-server_physics-desc = Уключыць/адключыць аўтарытэтную фізіку сервера для ўліковага запісу
command-set_motd-desc = Усталяваць апісанне сервера
command-set-waypoint-desc = Усталяваць шляхавую кропку ў адпаведнасці з вашым бягучым месцівам.
command-ship-desc = Стварыць карабель
command-site-desc = Тэлепартавацца ў пэўнае месца
command-skill_point-desc = Дадаць сабе скіл поінты для пэўнага дрэва навыкаў
command-skill_preset-desc = Даць вашаму персанажу патрэбныя навыкі.
command-spawn-desc = Стварыць тэставы аб'ект
command-spot-desc = Знайсці найблізкае месца пэўнага тыпу і тэлепартавацца туды.
command-sudo-desc = Запусціце каманду так, як быццам вы з'яўляецеся іншым суб'ектам
command-tell-desc = Адправіць паведамленне іншаму гульцу
command-tether-desc = Прывязаць да сябе іншую істоту
command-time-desc = Усталюйце час сутак
command-time_scale-desc = Усталяваць маштабаванне дэльта-часу
command-tp-desc = Тэлепартавацца да іншай істоты
command-rtsim_chunk-desc = Адлюстраваць інфармацыю пра бягучы чанк з rtsim
command-rtsim_info-desc = Адлюстраваць інфармацыю пра NPC у rtsim
command-rtsim_npc-desc = Вывесці спіс NPC у rtsim, адпаведных зададзенаму запыту (прыкладам: simulated, merchant), у парадку змяншэння адлегласці
command-rtsim_purge-desc = Ачысціць дадзеныя rtsim пры наступным запуску
command-rtsim_tp-desc = Тэлепартавацца да rtsim npc
command-unban-desc = Зняць бан з паказанага імя карыстача. Калі злучаны з ім IP-адрас таксама заблакаваны, блакаванне будзе знята і з яго.
command-unban-ip-desc = Зняць толькі IP-блакаванне для паказанага імя карыстача.
command-version-desc = Вывесці версію сервера
command-weather_zone-desc = Стварыць зону надвор'я
command-whitelist-desc = Дадаць/выдаліць імя карыстача з белага спіса
command-wiring-desc = Стварыць элемент падлучэння
command-world-desc = Адправіць паведамленні ўсім карыстачам сервера
command-wiki-desc = Адкрыць вікі ці знайсці патрэбную тэму
command-reset_tutorial-desc = Скінуць гульнёвы туторыял да пачатковага стану
command-reset_tutorial-success = Скінуць стан навучальнага дапаможніка.
command-naga-desc = Уключыць/выключыць выкарыстанне naga пры пачатковай апрацоўцы шэйдараў (налада не захоўваецца)
players-list-header =
    { $count ->
        [1]
            { $count } гулец анлайн
            { $player_list }
        [few]
            { $count } гульца анлайн
            { $player_list }
        [many]
            { $count } гулцой анлайн
            { $player_list }
       *[other]
            { $count } гульцой анлайн
            { $player_list }
    }
command-clear-desc = Выдаліць усе паведамленні ў чаце. Дзее на ўсе ўкладкі чату.
command-experimental_shader-desc = Уключыць/выключыць эксперыментальны шэйдар.
command-help-desc = Адлюстраваць інфармацыю пра каманды
command-mute-desc = Адключыць адлюстраванне паведамленняў у чаце ад дадзенага гульца.
command-unmute-desc = Зняць адключэнне гуку з прайгравальніка, гук якога быў адключаны з дапамогай каманды «mute».
command-waypoint-desc = Паказаць месціва бягучай шляхавой кропкі
command-preprocess-target-error = Чакалася { $expected_list } пасля знака «@», але знойдзена { $target }
command-preprocess-not-looking-at-valid-target = Не накіравана на сапраўдную мэту
command-preprocess-not-selected-valid-target = Не выбраны дапушчальны аб'ект
command-preprocess-not-valid-viewpoint-entity = Не бачна з дапушчальнага аб'екта пункту погляду
command-preprocess-not-riding-valid-entity = Не ўжываецца дапушчальны аб'ект
command-preprocess-not-valid-rider = Няма сапраўднай дадатковай умовы
command-preprocess-no-player-entity = Адсутнічае існасць гулец
command-invalid-command-message =
    Не атрымалася знайсці каманду з імем { $invalid-command }.
    Магчыма, вы мелі на ўвазе адну з наступных камандаў?
    { $most-similar-command }
    { $commands-with-same-prefix }

    Увядзіце /help, каб прагледзець спіс усіх камандаў.
command-mute-cannot-mute-self = Вы не можаце заглушыць сябе
command-mute-success = { $player } паспяхова заглушаны
command-mute-no-player-found = Не атрымалася знайсці гульца з імем { $player }
command-mute-already-muted = { $player } ужо заглушаны
command-mute-no-player-specified = Патрэбна азначыць гульца
command-unmute-cannot-unmute-self = Вы не можаце ўключыць гук сябе
command-unmute-success = Гульцу { $player } гук уключаны
command-unmute-no-muted-player-found = Не атрымалася знайсці гульца { $player } з адключаным гукам
command-unmute-no-player-specified = Патрэбна ўказаць гульца, якога трэба заглушыць
command-shader-backend = Бягучы бэкэнд шэйдара: { $shader-backend }
command-experimental-shaders-list = { $shader-list }
command-experimental-shaders-not-found = Эксперыментальных шэйдараў няма
command-experimental-shaders-enabled = { $shader } улучаны
command-experimental-shaders-disabled = { $shader } вылучаны
command-experimental-shaders-not-supported = { $shader } не падтрымваецца ў дадзенай зборцы гульні
command-experimental-shaders-not-a-shader = { $shader } не з'яўляецца эксперыментальным шэйдарам; скарыстайцеся гэтай камандай з любымі аргументамі, каб убачыць поўны спіс.
command-experimental-shaders-not-valid = Трэба ўказаць дапушчальны эксперыментальны шэйдар; каб атрымаць спіс эксперыментальных шэйдараў, выканайце гэту каманду без аргументаў.
command-no-permission = У вас няма дазволу на выкарыстанне '/{ $command_name }'
command-position-unavailable = Немагчыма атрымаць пазіцыю для { $target }
command-player-role-unavailable = Немагчыма атрымаць роль адміністратара для { $target }
command-uid-unavailable = Немагчыма атрымаць UID для { $target }
command-area-not-found = Немагчыма знайсці вобласць '{ $area }'
command-player-not-found = Гулец '{ $player }' не знойдзены!
command-player-uuid-not-found = Гулец з UUID '{ $uuid }' не знойдзены!
command-username-uuid-unavailable = Немагчыма вызначыць UUID для ўліковага запісу { $username }
command-uuid-username-unavailable = Немагчыма вызначыць імя карыстальніка дляUUID  { $uuid }
command-no-sudo = Нявыхавана ўдаваць з сябе іншых гульцоў
command-entity-dead = Істота '{ $entity }' мёртвы!
command-error-write-settings =
    Не ўдалося запісаць файл налад на дыск, але ўдалося ў памяць.
    Памылка (сховішча): { $error }
    Поспех (памяць): { $message }
command-error-while-evaluating-request = Падчас праверкі запыту адбылася памылка: { $error }
command-give-inventory-full =
    Інвентар гульца поўны. Выданы { $given ->
        [1] толькі адзін
       *[other] { $given }
    } з { $total } прадметаў.
command-give-inventory-success = Дададзена { $total } x { $item } у інвентар.
command-invalid-item = Няправільны прадмет: { $item }
command-invalid-block-kind = Няправільны тып блока: { $kind }
command-nof-entities-at-least = Сутнасцей павінна быць ня менш за 1
command-nof-entities-less-than = Сутнасцей павінна быць менш за 50
command-entity-load-failed = Не атрымалася загрузіць канфігурацыю аб'екта: { $config }
command-spawned-entities-config = З канфігурацыі { $config } створана { $n } аб'ектаў
command-invalid-sprite = Недазволены тып спрайта: { $kind }
command-time-parse-too-large = { $n } недазволены, не можа ўтрымваць больш 16 лічбаў.
command-time-parse-negative = { $n } недазволены, не можа быць адмоўным.
command-time-backwards = { $t } у мінулым, час не можа цячы ў зваротны бок.
command-time-invalid = { $t } гэта недазволены час.
command-time-current = Гэта { $t }
command-time-unknown = Час невядомы
command-rtsim-purge-perms = Вы павінны быць адміністратарам (не часовым), каб падаляць дадзеныя rtsim.
command-chunk-not-loaded = Чанк { $x }, { $y } не загружаны
command-chunk-out-of-bounds = Чанк { $x }, { $y } за межамі мапы
command-spawned-entity = Створана існасць з ID: { $id }
command-spawned-dummy = Створаны трэнеравальны манекен
command-spawned-airship = Створаны дырыжабль
command-spawned-campfire = Вогнішча распалена
command-spawned-safezone = Створана бяспечная зона
command-volume-size-incorrect = Значэнне павінна быць ад 1 да 127.
command-volume-created = Аб'ём створаны
command-permit-build-given = Вы не можаце будаваць у '{ $area }'
command-permit-build-granted = Выдадзены дазвол на будаванне ў '{ $area }'
command-revoke-build-recv = Ваш дазвол на будаванне ў «{ $area }» адкліканы
command-revoke-build = Дазвол на будаванне ў «{ $area }» адкліканы
command-revoke-build-all = Вашыя дазволы на будаванне ў адкліканы.
command-revoked-all-build = Усе дазволы на будаванне адкліканы.
command-no-buid-perms = У вас няма дазволу на будаванне.
command-set-build-mode-off = Рэжым будавання адключаны.
command-set-build-mode-on-persistent = Рэжым будавання ўлучаны. Улучана эксперыментальнае захаванне змен ландшафту. Сервер будзе спрабаваць захаваць змены, але іх захаванне не гарантуецца.
command-set-build-mode-on-unpersistent = Рэжым будавання ўлучаны. Змены будуць скінуты пасля перазагрузкі чанка.
command-set_motd-message-added = Паведамленне дня на серверы ўсталявана на { $message }
command-set_motd-message-removed = Паведамленне дня на серверы выдалена
command-set_motd-message-not-set = Для гэтай лакалі не было зададзена паведамленне дня (motd)
command-set-waypoint-result = Шляхавая кропка ўсталявана!
command-invalid-alignment = Няслушная прыналежнасць: { $alignment }
command-kit-not-enough-slots = Недастаткова слотаў у інвентары
command-lantern-unequiped = Калі ласка, спачатку вазьміце ліхтарык
command-lantern-adjusted-strength = Вы змянілі сілу полымя.
command-lantern-adjusted-strength-color = Вы змянілі сілу і колер полымя...
command-explosion-power-too-high = Магутнасць выбуху не можа перавышаць { $power }
command-explosion-power-too-low = Сіла выбуху павінна быць больш, чым { $power }
