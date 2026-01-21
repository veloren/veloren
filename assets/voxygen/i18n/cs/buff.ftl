buff-remove = Klikni pro zrušení
buff-heal = Heal
    .desc = Časem doplňuje zdraví.
    .stat =
        { $duration ->
            [1] Obnoví { $str_total } životů během { $duration } sekundy.
           *[other] Obnoví { $str_total } životů během { $duration } sekund.
        }
buff-potion = Lektvar
    .desc = Piju...
buff-saturation = Nasycení
    .desc = Přidá život během času ze Spotřebních.
buff-invulnerability = Nezranitelnost
    .desc = Žádný útok ti neublíží.
    .stat =
        { $duration ->
            [1]
                Poskytuje nezranitelnost.
                Trvá { $duration } sekundu.
            [few]
                Poskytuje nezranitelnost.
                Trvá { $duration } sekundy.
           *[other]
                Poskytuje nezranitelnost.
                Trvá { $duration } sekund.
        }
buff-protectingward = Ochraná Vizita
    .desc = Jsi chráněn, nějak, před útoky.
buff-frenzied = Šílený
    .desc = Jsi prostoupen nepřirozenou rychlostí a můžeš ignorovat drobná zranění.
buff-bleed = Krvácení
    .desc = Způsobuje pravidelné poranění.
buff-cursed = Prokletí
    .desc = Jsi prokletý.
buff-burn = V plamenech
    .desc = Hoříš zaživa.
buff-crippled = Zmrzačený
    .desc = Pohybuješ se jako mrzák, protože tvoje nohy jsou těžce poraněny.
buff-frozen = Zmražen
    .desc = Tvé pohyby a útoky jsou zpomaleny.
buff-wet = Mokrý
    .desc = Kloužou ti nohy, proto je obtížné zastavit.
buff-ensnared = Polapen
    .desc = Liány ti svazují nohy, takže se nemůžeš hýbat.
buff-increase_max_energy = Zvýšení Maximální Energie
    .desc = Maximum tvé energie se zvýší.
    .stat =
        { $duration ->
            [1]
                Zvyšuje maximální energii
                o { $strength }.
                Trvá { $duration } sekundu.
            [few]
                Zvyšuje maximální energii
                o { $strength }.
                Trvá { $duration } sekundy.
           *[other]
                Zvyšuje maximální energii
                o { $strength }.
                Trvá { $duration } sekund.
        }
buff-increase_max_health = Zvýšení zdraví
    .desc = Tvá hodnota maximálního zdraví je zvýšena.
    .stat =
        { $duration ->
            [1]
                Zvyšuje maximum životů
                o { $strength }.
                Trvá { $duration } sekundu.
            [few]
                Zvyšuje maximum životů
                o { $strength }.
                Trvá { $duration } sekundy.
           *[other]
                Zvyšuje maximum životů
                o { $strength }.
                Trvá { $duration } sekund.
        }
buff-scornfultaunt = Pohrdavý Výsměch
    .desc = Pohrdavě se vysmíváš nepřátelům, což ti posiluje statečnost a výdrž. Ale tvoje smrt posílí tvého vraha.
buff-winded = Bez dechu
    .desc = Sotva dýcháš, což omezuje kolik energie můžeš obnovit a jak rychle se můžeš hýbat.
buff-rooted = Zakořenění
    .desc = Jsi zaseklý na místě a nemůžeš se hýbat.
buff-staggered = Omráčení
    .desc = Jsi rozhozený a zranitelnější těžkými útoky.
buff-mysterious = Záhadný efekt
buff-fury = Zuřivost
    .desc = V návalu vzteku tvoje zásahy generují větší kombo.
buff-defiance = Vzdor
    .desc = Můžeš ustát mocnější a silněji omračující zásahy a generovat kombo při obdržení zásahu, ale jsi pomalejší.
buff-lifesteal = Ukradený život
    .desc = Vysaje z nepřátel jejich život.
buff-heatstroke = Úpal
    .desc = Byl jsi vystaven horku a nyní máš úpal. Tvoje odměna energie a rychlost pohybu jsou osekány. Vychladni.
buff-agility = Hbitost
    .desc =
        Pohybuješ se rychleji,
        ale udílíš méně a dostáváš více poškození.
    .stat =
        { $duration ->
            [1]
                Zvyšuje rychlost o { $strength } %.
                Výměnou za to se drasticky sníží tvůj útok a obrana.
                Trvá { $duration } sekundu.
            [few]
                Zvyšuje rychlost o { $strength } %.
                Výměnou za to se drasticky sníží tvůj útok a obrana.
                Trvá { $duration } sekundy.
           *[other]
                Zvyšuje rychlost o { $strength } %.
                Výměnou za to se drasticky sníží tvůj útok a obrana.
                Trvá { $duration } sekund.
        }
buff-energy_regen = Obnova energie
    .desc = Rychlejší obnovení energie.
    .stat =
        { $duration ->
            [1] Obnoví { $str_total } energie během { $duration } sekundy.
           *[other] Obnoví { $str_total } energie během { $duration } sekund.
        }
buff-hastened = Zrychlení
    .desc = Tvoje pohyby a útoky jsou rychlejší.
buff-poisoned = Otrávený
    .desc = Cítíš, jak tvůj život uvadá...
buff-fortitude = Odolnost
    .desc = Dokážeš odolat omráčení a čím více dostaneš poškození, tím snáz ostatní omráčíš.
buff-parried = Odražen
    .desc = Byl jsi odražen a nyní se pomalu vzpamatováváš.
buff-potionsickness = Nevolnost z lektvarů
    .desc = Poslední vypitý lektvar způsobí, že každý další bude mít menší efekt.
    .stat =
        { $duration ->
            [1]
                Snižuje pozitivní efekty
                dalších lektvarů o { $strength } %.
                Trvá { $duration } sekundu.
            [few]
                Snižuje pozitivní efekty
                dalších lektvarů o { $strength } %.
                Trvá { $duration } sekundy.
           *[other]
                Snižuje pozitivní efekty
                dalších lektvarů o { $strength } %.
                Trvá { $duration } sekund.
        }
buff-reckless = Lehkomyslnost
    .desc = Tvoje útoky jsou silnější. Ale necháváš svou obranu otevřenou.
buff-polymorphed = Polymorfní
    .desc = Tvé tělo mění formu.
buff-frigid = Zmrzlý
    .desc = Zmrazí tvé nepřátele.
buff-salamanderaspect = Vlastnost Mloka
    .desc = Nemůžeš uhořet a lávou se pohybuješ rychleji.
buff-imminentcritical = Nevyhnutelně Kritický
    .desc = Tvůj další útok kriticky zasáhne nepřítele.
buff-sunderer = Podrobení
    .desc = Tvé útoky mohou prorazit obranu nepřítele a obnoví ti více energie.
buff-bloodfeast = Krvavá hostina
    .desc = Útoky proti krvácejícímu nepříteli ti obnovují životy.
buff-berserk = Nepříčetnost
    .desc = Jsi nepříčetný, címž jsou tvé útoky mocnější a tvé pohyby i útoky rychlejší. Ale výsledkem je snížená schopnost tvé obrany.
buff-concussion = Otřes mozku
    .desc = Dostal jsi silnou ránu do hlavy a máš problém se soustředit, to ti zabraňuje používat komplexnější útoky.
buff-tenacity = Houževnatost
    .desc = Nejen že jsi schopen odrazit těžší útoky, ale také tě nabíjí energií. Ale jsi zároveň pomalejší.
buff-resilience = Odolnost
    .desc = Po zásahu oslabujícím útokem jsi odolnější proti dalším oslabujícím efektům.
buff-resting_heal = Léčení odpočinkem
    .desc = Odpočinek obnoví { $rate } % zdraví za sekundu.
buff-combo_generation = Tvorba komba
    .desc = S ubíhajícím časem vytváří kombo.
    .stat =
        { $duration ->
            [1] Vytváří { $str_total } kombo za { $duration } sekund.
            [few] Vytváří { $str_total } komba za { $duration } sekund.
           *[other] Vytváří { $str_total } komb za { $duration } sekund.
        }
buff-owltalon = Soví Spár
    .desc = Díky tomu, že tvůj cíl vůbec neví o tvé přítomnosti, bude tvůj útok přesnější a silnější.
buff-heavynock = Těžký Šíp
    .desc = Nasaď si do luku těžší šíp, čímž budeš schopný následující střelou ochromit cíl. Těžší šíp bude mít ale menší průbojnost na dlouhé vzdálenosti.
buff-eagleeye = Oko Dravce
    .desc = Přesně vidíš zranitelná místa nepřátel a máš dost obratnosti k vedení každého šípu do těchto míst.
buff-chilled = Vymrzlý
    .desc = Intenzivní chlad tě zpomaluje a jsi zranitelnější silovým útokům.
buff-ardenthunter = Zanícený Lovec
    .desc = Tvá horlivost dělá tvé střely smrtelnějšími pro konkrétní cíl. A tvá energie roste s každým zásahem.
buff-ardenthunted = Zanícený Lovec
    .desc = Byl jsi označen jako cíl horlivého lučišníka.
buff-septicshot = Hnisající Rána
    .desc = Tvůj další šíp způsobí cíli infekci, která bude ještě účinnější, pokud má cíl nějaké další stavy.
buff-heartseeker = Srdcobijec
    .desc = Tvůj další šíp udeří na nepřítele jako by trefil přímo do srdce, tím mu způsobí vážnější zranění a tobě přidá energii.
