## Regeneration

buff-heal = Läk
    .desc = Återfå liv över tid.
    .stat =
        { $duration ->
            [1] Återger { $str_total } livpoäng under { $duration } sekund.
           *[other] Återger { $str_total } livpoäng under { $duration } sekunder.
        }

## Potion

buff-potion = Trolldryck
    .desc = Dricker...

## Saturation

buff-saturation = Mättnad
    .desc = Återfå hälsa över tid från förbrukningsvaror.

## Campfire


## Energy Regen

buff-energy_regen = Energiregenerering
    .desc = Snabbare energiregenerering..
    .stat =
        { $duration ->
            [1] Återställer { $str_total } energi över { $duration } sekund.
           *[other] Återställer { $str_total } energi över { $duration } sekunder.
        }

## Health Increase

buff-increase_max_health = Öka Maximala Hälsan
    .desc = Din maximala hälsogräns är ökad.
    .stat =
        Ökar den maximala hälsan
        med { $strength }.

## Energy Increase

buff-increase_max_energy = Öka Maximala Energin
    .desc = Höj din maximala energigräns.
    .stat =
        Ökar den maximala energin
        med { $strength }.

## Invulnerability

buff-invulnerability = Osårbarhet
    .desc = Du kan inte skadas av någon attack.
    .stat = Ger osårbarhet.

## Protection Ward

buff-protectingward = Skyddsbesvärjelse
    .desc = Du skyddas, någorlunda, från attacker.

## Frenzied

buff-frenzied = Rasande
    .desc = Du är uppfylld av en onaturlig hastighet och kan ignorera mindre skador.

## Haste

buff-hastened = Förhastad
    .desc = Dina rörelser och attacker är snabbare.

## Bleeding

buff-bleed = Blödande
    .desc = Orsakar regelbunden skada.

## Curse

buff-cursed = Förbannad
    .desc = En förbannelse har uttalats över dig.

## Burning

buff-burn = I Lågor
    .desc = Du brinner levande.

## Crippled

buff-crippled = Förlamad
    .desc = Din rörlighet hindras eftersom dina ben är allvarligt skadade.

## Freeze

buff-frozen = Frusen
    .desc = Dina rörelser och attacker har saktats ner.

## Wet

buff-wet = Blöt
    .desc = Marken nekar dina fötter, vilket gör det svårt att stanna.

## Ensnared

buff-ensnared = Intrasslad
    .desc = Rankor greppar tag i dina ben vilket begränsar din rörlighet.

## Fortitude

buff-fortitude = Slagtålig
    .desc = Du kan tåla vacklande attacker, och när du tar med skada kan du enklare få andra att vackla.

## Parried

buff-parried = Parerad
    .desc = Du parerades och är nu långsam med att återhämta dig.

## Potion sickness

buff-potionsickness = Trolldryckssjuka
    .desc = Trolldryck har mindre positiva effekter på dig efter nyligt konsumerade trolldryck.
    .stat =
        { $duration ->
            [1]
                Minskar de positiva effekterna av
                följande trolldryck med { $strength } %.
                Varar i { $duration } sekund.
           *[other]
                Minskar de positiva effekterna av
                följande trolldryck med { $strength } %.
                varar i { $duration } sekunder.
        }

## Reckless

buff-reckless = Hänsynslös
    .desc = Dina attacker är kraftfullare. Dock lämnar du dina försvar öppna.

## Polymorped

buff-polymorphed = Polymorferad
    .desc = Din kropp byter form.

## Flame


## Frigid

buff-frigid = Frusen
    .desc = Frys dina fiender.

## Lifesteal

buff-lifesteal = Livsstöld
    .desc = Sug ut livet ur dina fiender.

## Polymorped

buff-salamanderaspect = Salamanders Aspekt
    .desc = Du kan inte brinna och du rör dig snabbt genom lava.

## Imminent Critical

buff-imminentcritical = Inkommande Kritisk Träff
    .desc = Din nästa attack kommer ge ett kritiskt slag mot fienden.

## Fury

buff-fury = Raseri
    .desc = Med ditt raseri genererar dina hugg mer kombo.

## Sunderer

buff-sunderer = Söndrare
    .desc = Dina attacker bryter igenom dina fienders försvar och friskar upp dig med mer energi.

## Sunderer

buff-defiance = Trotsning
    .desc = Du kan stå emot starkare och mer omskakande slag och generera kombo genom att bli slagen. Däremot är du långsammare.

## Bloodfeast

buff-bloodfeast = Blodfest
    .desc = Du fyller upp ditt liv när du attackerar blödande fiender.

## Berserk

buff-berserk = Bärsärk
    .desc = Du går bärsärkagång. Dina attacker blir skadligare och både du och dina attacker blir snabbare. Däremot blir dina defensiva förmågor svagare.

## Util

buff-mysterious = Mystisk effekt
buff-remove = Klicka för att ta bort
# Agility
buff-agility = Smidighet
    .desc =
        Din rörelse är snabbare,
        men du gör minder skada och tar mer skada.
    .stat =
        { $duration ->
            [1]
                Ökar rörelsehastighet med { $strength } %.
                I utbyte minskar din attackkraft och skydd drastiskt.
                varar i { $duration } sekund.
           *[other]
                Ökar rörelsehastighet med { $strength } %.
                I utbyte minskar din attackkraft och skydd drastiskt.
                Varar i { $duration } sekunder.
        }
# Heatstroke
buff-heatstroke = Värmeslag
    .desc = Du utsattes för värme och lider nu av värmeslag. Din energibelöning och rörelsehastighet är lägre. Chilla.
# Poisoned
buff-poisoned = Förgiftad
    .desc = Du känner ditt liv vissna iväg...
# Concussion
buff-concussion = Hjärnskakning
    .desc = Du har blivit slagen hårt på huvudet och har svårt att fokusera, vilket förhindrar dig från att använda några av dina mer komplexa attacker.
# Tenacity
buff-tenacity = Tenacitet
    .desc = Du skakar inte bara av tyngre attacker, de ger dig även energi. Dock är du också saktare.
# Winded
buff-winded = Andfådd
    .desc = Du kan knappt andas vilket hindrar hur mycket energi du kan återhämta och hur snabbt du kan röra dig.
# Rooted
buff-rooted = Rotad
    .desc = Du sitter fast på plats och kan inte röra dig.
# Staggered
buff-staggered = Vacklan
    .desc = Du är i obalans och mer mottaglig till tunga attacker.
buff-resilience = Resilians
    .desc = Efter att just ha tagit en försvagande attack blit du mer resilient mot framtida försvagande effekter.
buff-scornfultaunt = Föraktfullt Hån
    .desc = Du hånar dina fiender föraktfullt, och får förstärkt mod och ork. Din död kommer dock förstärka din dödare.
buff-combo_generation = Kombogenerering
    .desc = Genererar kombo över tid.
    .stat =
        { $duration ->
            [1] Genererar { $str_total } kombo under { $duration } sekund.
           *[other] Genererar { $str_total } kombo under { $duration } sekunder.
        }
