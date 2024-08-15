## Regeneration

buff-heal = Лечение
    .desc = Постепенное восстановление здоровья
    .stat = Восстанавливает { $str_total } здоровья

## Potion

buff-potion = Зелье
    .desc = Питьё...

## Saturation

buff-saturation = Насыщение
    .desc = Восстановление здоровья за счет расходных материалов.

## Campfire

buff-campfire_heal = Исцеление у костра
    .desc = Отдых у костра лечит { $rate }% в секунду.

## Energy Regen

buff-energy_regen = Восстановление энергии
    .desc = Ускоренное восстановление энергии
    .stat = Восстанавливает { $str_total } энергии

## Health Increase

buff-increase_max_health = Повышение максимального здоровья
    .desc = Увеличение лимита здоровья
    .stat =
        Повышает максимум здоровья
        на { $strength }

## Energy Increase

buff-increase_max_energy = Повышение максимальной энергии
    .desc = Увеличение лимита энергии
    .stat =
        Повышает максимум энергии
        на { $strength }

## Invulnerability

buff-invulnerability = Неуязвимость
    .desc = Вы не можете получить урон от атак.
    .stat = Дарует неуязвимость

## Protection Ward

buff-protectingward = Защитная Аура
    .desc = Вы в некоторой степени защищены от атак.

## Frenzied

buff-frenzied = Бешенство
    .desc = Кровь течёт быстрее, ускоряя ваше движение и понемногу исцеляя вас.

## Haste

buff-hastened = Ускорение
    .desc = Скорость передвижения и атак повышена.

## Bleeding

buff-bleed = Кровотечение
    .desc = Наносит регулярный урон.

## Curse

buff-cursed = Проклятие
    .desc = Вас прокляли.

## Burning

buff-burn = В огне
    .desc = Вы горите живьём

## Crippled

buff-crippled = Увечье
    .desc = Ваше движение затруднено, так как ваши ноги сильно травмированы.

## Freeze

buff-frozen = Обморожение
    .desc = Скорость движения и атак снижена.

## Wet

buff-wet = Мокрый
    .desc = Ваши ноги не слушаются, остановка затруднена.

## Ensnared

buff-ensnared = Ловушка
    .desc = Лоза опутывает ваши ноги затрудняя движение.

## Fortitude

buff-fortitude = Стойкость
    .desc = Вы можете выдерживать оглушающие удары.

## Parried

buff-parried = Парированный
    .desc = Вашу атаку отразили, ваше восстановление замедлено.

## Potion sickness

buff-potionsickness = Отравление зельем
    .desc = Зелья исцеляют вас меньше, если вы недавно уже употребили другое зелье.
    .stat =
        Уменьшает исцеление от
        последующих зелий на { $strength }%.

## Reckless

buff-reckless = Безрассудный
    .desc = Ваши атаки стали сильнее, однако вы стали открытым для вражеских атак.

## Util

buff-text-over_seconds =
    более { $dur_secs ->
        [one] секунды
       *[other] { $dur_secs } секунд
    }
buff-text-for_seconds =
    на { $dur_secs ->
        [one] { $dur_secs } секунду
        [few] { $dur_secs } секунды
        [many] { $dur_secs } секунд
       *[other] { $dur_secs } секунд
    }
buff-remove = Нажмите, чтобы удалить
# Imminent Critical
buff-imminentcritical =
    .desc = Ваша следующая атака нанесет противнику критический удар.
buff-mysterious = Таинственный эффект
# Polymorped
buff-polymorphed =
    .desc = Ваше тело меняет форму.
# Fury
buff-fury = Ярость
# Frigid
buff-frigid =
    .desc = Заморозьте своих врагов.
# Berserk
buff-berserk = Берсерк
    .desc = Вы находитесь в состоянии ярости, в результате чего ваши атаки становятся более мощными и быстрыми, а скорость увеличивается. Однако при этом снижается способность к защите.
# Bloodfeast
buff-bloodfeast = Кровавый пир
# Salamander's Aspect
buff-salamanderaspect =
    .desc = Вы не горите и быстро перемещаетесь по лаве.
# Agility
buff-agility = Ловкость
    .desc = Вы двигаетесь быстрее, но наносите меньше урона и получаете больше повреждений.
    .stat =
        Увеличивает скорость передвижения на { $strength }%.
        но уменьшает ваш урон на 100%,
        и увеличивает уязвимость к урону
        на 100%.
