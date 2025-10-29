hud-do_not_show_on_startup = Tega ne pokaži ob zagonu
hud-show_tips = Pokaži namige
hud-quests = Poslanstva
hud-waypoint_saved = Kažipot shranjen
hud-inventory_full = Inventar je poln
hud-press_key_to_show_keybindings_fmt = { "[" }{ $key }] Nastavitve tipk
hud-press_key_to_toggle_lantern_fmt = { "[" }{ $key }] Svetilka
hud-press_key_to_show_debug_info_fmt = Razhroščevalske informacije prikažeš s pritiskom na { $key }
hud-tutorial_btn = Učna ura
hud-tutorial_click_here = Pritisni [ { $key } ], da odkleneš kazalec miške in klikneš na ta gumb!
hud-tutorial_elements = Sestavljanje
hud-spell = Uroki
hud-diary = Dnevnik
hud-activate = Sproži
hud-collect = Zberi
hud-pick_up = Poberi
hud-open = Odpri
hud-steal = Ukradi
hud-use = Uporabi
hud-read = Preberi
hud-mine = Rudari
hud-dig = Koplji
hud-mine-needs_pickaxe = Zahteva kramp
hud-mine-needs_shovel = Zahteva lopato
hud-mine-needs_unhandled_case = Zahteva ???
hud-talk = Govori
hud-help = Pomagaj
hud-pet = Pobožaj
hud-trade = Trguj
hud-mount = Zajahaj
hud-follow = Sledi
hud-stay = Ostani
hud-sit = Sedi
hud-waypoint_interact = Nastavi kažipot
hud-steer = Usmerjaj
hud-rest = Počivaj
hud-portal = Portal
-server = Strežnik
-client = Odjemalec
hud-init-stage-singleplayer = Zaganjam enoigralski strežnik ...
hud-init-stage-server-db-migrations = { "[" }{ -server }]: Uveljavljam migracije podatkovne baze ...
hud-init-stage-server-db-vacuum = { "[" }{ -server }]: Čistim podatkovno bazo ...
hud-init-stage-server-worldsim-erosion = { "[" }{ -server }]: Erozija { $percentage } %
hud-init-stage-server-worldsim-erosion_time_left =
    .days =
        { $n ->
            [one] ostaja še ~{ $n } dan
            [two] ostajata še ~{ $n } dneva
            [few] ostajajo še ~{ $n } dnevi
           *[other] ostaja še ~{ $n } dni
        }
    .hours =
        { $n ->
            [one] ostaja še ~{ $n } ura
            [two] ostajata še ~{ $n } uri
            [few] ostajajo še ~{ $n } ure
           *[other] ostaja še ~{ $n } ur
        }
    .minutes =
        { $n ->
            [one] ostaja še ~{ $n } minuta
            [two] ostajata še ~{ $n } minuti
            [few] ostajajo še ~{ $n } minute
           *[other] ostaja še ~{ $n } minut
        }
    .seconds =
        { $n ->
            [one] ostaja še ~{ $n } sekunda
            [two] ostajata še ~{ $n } sekundi
            [few] ostajajo še ~{ $n } sekunde
           *[other] ostaja še ~{ $n } sekund
        }
hud-init-stage-server-worldciv-civcreate =
    { "[" }{ -server }]: { $generated ->
        [one] Generirana { $generated }
        [two] Generirani { $generated }
        [few] Generirane { $generated }
       *[other] Generiranih { $generated }
    } od { $total ->
        [one] { $total } civilizacije
       *[one] { $total } civilizacij
    }
hud-init-stage-server-worldciv-site = { "[" }{ -server }]: Generiram kraje ...
hud-init-stage-server-economysim = { "[" }{ -server }]: Simuliram gospodarstvo ...
hud-init-stage-server-spotgen = { "[" }{ -server }]: Generiram prostore ...
hud-init-stage-server-starting = { "[" }{ -server }]: Zaganjam strežnik ...
hud-init-stage-multiplayer = Zaganjam večigralsko igro
hud-init-stage-client-connection-establish = { "[" }{ -client }]: Vzpostavljam povezavo s strežnikom ...
hud-init-stage-client-request-server-version = { "[" }{ -client }]: Čakam na različico strežnika ...
hud-init-stage-client-authentication = { "[" }{ -client }]: Overjam ...
hud-init-stage-client-load-init-data = { "[" }{ -client }]: Nalagam zagonske podatke s trežnika ...
hud-init-stage-client-starting-client = { "[" }{ -client }]: Pripravljam odjemalca ...
hud-init-stage-render-pipeline = Sestavljam cevovod upodabljanja ({ $done }/{ $total })
