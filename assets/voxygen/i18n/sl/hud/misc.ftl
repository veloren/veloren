hud-show_tips = Pokaži namige
hud-quests = Naloge
hud-waypoint_saved = Kažipot shranjen
hud-inventory_full = Inventar je poln
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
       *[other] { $total } civilizacij
    }
hud-init-stage-server-worldciv-site = { "[" }{ -server }]: Generiram kraje ...
hud-init-stage-server-economysim = { "[" }{ -server }]: Simuliram gospodarstvo ...
hud-init-stage-server-spotgen = { "[" }{ -server }]: Generiram prostore ...
hud-init-stage-server-starting = { "[" }{ -server }]: Zaganjam strežnik ...
hud-init-stage-multiplayer = Zaganjam večigralsko igro
hud-init-stage-client-connection-establish = { "[" }{ -client }]: Vzpostavljam povezavo s strežnikom ...
hud-init-stage-client-request-server-version = { "[" }{ -client }]: Čakam na različico strežnika ...
hud-init-stage-client-authentication = { "[" }{ -client }]: Overjam ...
hud-init-stage-client-load-init-data = { "[" }{ -client }]: Nalagam zagonske podatke s strežnika ...
hud-init-stage-client-starting-client = { "[" }{ -client }]: Pripravljam odjemalca ...
hud-init-stage-render-pipeline = Sestavljam cevovod upodabljanja ({ $done }/{ $total })
hud-sp_arrow_txt = TV
hud-deactivate = Deaktiviraj
hud-you_died = Umrl_a si
hud-someone_else = Nekdo drug
hud-another_group = Druga skupina
hud-owned_by_for_secs =
    { $secs ->
        [one] { $name } ima to v lasti že { $secs } sekundo
        [two] { $name } ima to v lasti že { $secs } sekundi
        [few] { $name } ima to v lasti že { $secs } sekunde
       *[other] { $name } ima to v lasti že { $secs } sekund
    }
hud-press_key_to_toggle_keybindings_fmt = Bližnjice preklapljaš s tipko { $key }
hud-press_key_to_toggle_debug_info_fmt = Razhroščevalske infomacije preklopiš s tipko { $key }
hud-items_lost_dur = Tvoji pripravljeni predmeti so izgubili trpežnost.
hud-items_will_lose_dur = Tvoji pripravljeni predmeti bodo izgubili trpežnost.
hud-hardcore_char_deleted = Ta nepovratni lik je bil izbrisan.
hud-hardcore_will_char_deleted = Ta nepovratni lik bo izbrisan.
hud-press_key_to_respawn = Pritisni { $key }, da se pojaviš ob zadnjem obiskanem tabornem ognju.
hud-press_key_to_give_up = Pritisni in drži { $key }, da se vdaš in umreš.
hud-press_key_to_return_to_char_menu = Pritisni { $key }, da se vrneš na meni za like.
hud-downed_recieving_help = Pomoč prihaja.
hud-temp_quest_headline = Pozdravljen_a, popotnik_ca!
hud-temp_quest_text =
    Svojo pustolovščino lahko pričneš tako, da se razgledaš po vasi in si nabereš potrebščin.

    Vzameš lahko karkoli, kar ti bo koristilo na tvojem popotovanju!

    Na spodnji desni strani zaslona najdeš svojo torbo, sestavljalni meni in zemljevid.

    Na sestavljalnih postajah lahko ustvariš oklep, orožja, hrano in še marsikaj drugega!

    Iz kožuha divjih živali okoli vasice si lahko ustvariš zaščito pred nevarnostmi, ki prežijo tam zunaj.

    Kadar boš nared, lahko opraviš katerega od številnih izzivov, označenih na svojem zemljevidu, in tako dobiš še boljšo opremo!
hud-free_look_indicator =
    { $toggle ->
        [0] V prostem pogledu. Pritisni { $key }, da ga izklopiš.
       *[other] V prostem pogledu. Izpusti { $key }, da ga izklopiš.
    }
hud-camera_clamp_indicator = Kamera je navpično priklenjena. Pritisni { $key }, da jo sprostiš.
hud-auto_walk_indicator = Samodejna hoja/plavanje vklopljena
hud-zoom_lock_indicator-remind = Povečava zaklenjena
hud-zoom_lock_indicator-enable = Povečava kamere zaklenjena
hud-zoom_lock_indicator-disable = Povečava kamere odklenjena
hud-unlock-requires = Odpri z { $item }
hud-steal-requires = Ukradi z { $item }
hud-unlock-consumes = Porabi { $item } in odpri
hud-steal-consumes = Porabi { $item } in ukradi
hud-tutorial-disable = Trajno onemogoči učne namige
