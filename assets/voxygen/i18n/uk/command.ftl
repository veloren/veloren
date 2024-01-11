command-no-permission = Ви не маєте прав використовувати '/{ $command_name }'
command-position-unavailable = Неможливо отримати позицію для { $target }
command-player-role-unavailable = Неможливо отримати ролі адміністратора для { $target }
command-uid-unavailable = Неможливо отримати uid для { $target }
command-area-not-found = Неможливо знайти область '{ $area }'
command-player-not-found = Гравець '{ $player }' не знайдений!
command-player-uuid-not-found = Гравець з UUID '{ $uuid }' не знайдений!
command-username-uuid-unavailable = Неможливо визначити UUID для логіну { $username }
command-uuid-username-unavailable = Неможливо визначити логін для UUID  { $uuid }
command-no-sudo = Вдавати інших гравців неввічливо
command-entity-dead = Сутність '{ $entity }' мертва!
command-error-while-evaluating-request = При валідації запиту сталася помилка: { $error }
command-give-inventory-full = Інвентар гравця повний. Видано { $given ->
  [1] лише один
  *[other] { $given }
} з { $total } предметів.
command-invalid-item = Неправильний предмет: { $item }
command-invalid-block-kind = Неправильний тип блока: { $kind }
command-nof-entities-at-least = Сутностей має бути щонайменше 1
command-nof-entities-less-than = Сутностей має бути менше 50
command-entity-load-failed = Не вдалося завантажити конфігурацію сутності: { $config }
command-spawned-entities-config = Створено { $n } сутностей з конфігурації: { $config }
command-invalid-sprite = Неправильний тип спрайта: { $kind }
command-time-parse-too-large = { $n } невалідний, не може мати більше 16 цифр.
command-time-parse-negative = { $n } невалідний, не може бути від'ємний.
command-time-backwards = { $t } в минулому, час не може йти навпаки.
command-time-invalid = { $t } невалідний час.
command-rtsim-purge-perms = Ви маєте бути адміністратором (не тимчасовим) щоб видаляти дані rtsim.
command-chunk-not-loaded = Чанк { $x }, { $y } не завантажений
command-chunk-out-of-bounds = Чанк { $x }, { $y } за межами карти
command-spawned-entity = Створено сутність з ID: { $id }
command-spawned-dummy = Створено тренувальний манекен
command-spawned-airship = Створено повітряний корабель
command-spawned-campfire = Створено багаття
command-spawned-safezone = Створено безпечну зону
command-volume-size-incorrect = Розмір повинен бути від 1 до 127.
command-volume-created = Створено об'єм
command-permit-build-given = Ви не можете будувати в '{ $area }'
command-permit-build-granted = Надано дозвіл на будування в '{ $area }'
command-revoke-build-recv = Дозвіл на будування в '{ $area }' відкликано
command-revoke-build = Дозвіл будувати в '{ $area }' відкликано
command-revoke-build-all = Ваші дозволи на будування відкликано
command-revoked-all-build = Всі дозволи на будування відкликано
command-no-buid-perms = Ви не маєте дозволу будувати.
command-set-build-mode-off = Режим будівництва вимкнено.
command-set-build-mode-on-persistent = Режим будівництва увімкнуто. Ввімкнено експериментальне збереження змін ландшафту. Сервер намагатиметься зберегти зміни, але збереження не гарантується.
command-set-build-mode-on-unpersistent = Режим будівництва увімкнуто. Зміни пропадуть після перезавантаження чанка.
command-invalid-alignment = Неправильне вирівнювання: { $alignment }
command-kit-not-enough-slots = Недостатньо слотів в інвентарі
command-lantern-unequiped = Будь ласка, спочатку візьміть ліхтар
command-lantern-adjusted-strength = Ви змінили силу полум'я.
command-lantern-adjusted-strength-color = Ви змінили силу та колір полум'я..
command-explosion-power-too-high = Сила вибуху не може перевищувати { $power }
command-explosion-power-too-low = Сила вибуху повинна бути більше за { $power }
# Note: Do not translate "confirm" here
command-disconnectall-confirm = Виконайте, будь ласка, команду ще раз вказавши "confirm" як другий аргумент, щоб відключити всіх гравців.
command-invalid-skill-group = { $group } не група вмінь!
command-unknown = Невідома команда
command-disabled-by-settings = Команда вимкнена в налаштуваннях
command-battlemode-intown = Ви повинні бути в місті, щоб змінити бойовий режим!
command-battlemode-cooldown = Період очікування активний. Спробуйте ще раз через { $cooldown } секунд.
command-battlemode-available-modes = Доступні режими: pvp, pve
command-battlemode-same = Спроба встановити той самий бойовий режим.
command-battlemode-updated = Новий бойовий режим: { $battlemode }
command-buff-unknown = Невідомий баф: { $buff }
command-skillpreset-load-error = Помилка при завантаженні пресетів
command-skillpreset-broken = Пресет вмінь пошкоджений 
command-skillpreset-missing = Пресет не існує: { $preset }
command-location-invalid = Назва локації '{ $location }' невалідна. Назви можуть містити тільки малі літери ASCII та підкреслення. 
command-location-duplicate = Локація '{ $location }' вже існує.
command-location-not-found = Локація '{ $location }' не існує
command-location-created = Свторено локацію '{ $location }'
command-location-deleted = Видалено локацію '{ $location }'
command-locations-empty = Зараз не має локацій
command-locations-list = Доступні локації: { $locations }
# Note: Do not translate these weather names
command-weather-valid-values = Допустимі значення: 'clear', 'rain', 'wind', 'storm'
command-scale-set = Встановити масштаб в { $scale }
command-repaired-items = Всі одягнути речі відремонтовано
command-message-group-missing = Ви використовуєте чат групи, до якої не належите. Використовуйте /world чи
  /region щоб змінити чат.
command-tell-request = { $sender } хоче говорити з вами.

# Unreachable/untestable but added for consistency

command-player-info-unavailable = Неможливо отримати інформацію про гравця для { $target }
command-unimplemented-waypoint-spawn = Створення точки шляху не реалізовано
command-unimplemented-teleporter-spawn = Створення телепорта не реалізовано
command-kit-inventory-unavailable = Неможливо отримати інвентар
command-inventory-cant-fit-item = Предмет не вміщується в інвентар
# Emitted by /disconnect_all when you dont exist (?)
command-you-dont-exist = Ви не існуєте, отже не можете використовувати цю команду
command-destroyed-tethers = Всі пута знищено! Тепер ви вільні
command-destroyed-no-tethers = Ви не підключені до пут
command-dismounted = Ви спішилися
command-no-dismount = Ви не їдете верхи і ніхто не їде верхи на вас