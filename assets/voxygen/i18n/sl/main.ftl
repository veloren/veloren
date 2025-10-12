main-usernamevvvv = Uporabniško ime
main-server = Strežnik
main-password = Geslo
main-connecting = Povezujem
main-creating_world = Ustvarjam svet
main-tip = Namig:
main-unbound_key_tip = ni nastavljeno
main-notice =
    Dobrodošel_a v alfa različico Velorena!

    Preden se prepustiš zabavi, še nekaj pripomb.

    - Ta igra je še v zgodnji fazi razvoja in se ves čas dopolnjuje, zato pričakuj hroščatost, nedokončane ali okorne igralne mehanike in manjkajoče funkcionalnosti.

    - Če želiš podati konstruktivne pripombe ali prijaviti napake, nam lahko javiš na naš GitLab repozitorij, na Discord ali na Matrix.

    - Veloren je odprtokoden. Igro lahko prostor igraš, spreminjaš in deliš naprej v skladu z različico 3 Splošne javne licence GNU.

    - Veloren je neprofiten javni projekt. Vsi, ki se ukvarjamo z njim, smo prostovoljci.
    Če ti je naše delo všeč, te vabimo, da se pridružiš kateri od naših delovnih skupin!

    Hvala, da si si vzel_a čas in prebral_a tale zapisek. Upamo, da boš užival_a v igri!

    ~ Razvijalska ekipa
main-login_process =
    O večigralskem načinu:

    Za igranje na strežnikih, ki overjajo igralce, potrebuješ račun.

    Račun lahko ustvariš tukaj:
    https://veloren.net/account/
main-singleplayer-new = Novo
main-singleplayer-delete = Izbriši
main-singleplayer-regenerate = Ponovno generiraj
main-singleplayer-create_custom = Ustvari po meri
main-singleplayer-seed = Seme
main-singleplayer-day_length = Dan traja
main-singleplayer-random_seed = Naključno
main-singleplayer-size_lg = Logaritemska velikost
main-singleplayer-map_large_warning = Pozor, prvi vstop v velike svetove lahko traja dlje časa.
main-singleplayer-map_large_extra_warning = 
    { $count ->
        [one] To bi zahtevalo približno toliko virov kot generiranje { $count } sveta s privzetimi možnostmi.
        *[other] To bi zahtevalo približno toliko virov kot generiranje { $count } svetov s privzetimi možnostmi.
    }
main-singleplayer-world_name = Ime sveta
main-singleplayer-map_scale = Navpično skaliranje
main-singleplayer-map_erosion_quality = Kakovost erozije
main-singleplayer-map_shape = Oblika
main-singleplayer-map_shape-circle = Krog
main-singleplayer-map_shape-square = Kvadrat
main-singleplayer-play = Igraj
main-singleplayer-generate_and_play = Generiraj in igraj
menu-singleplayer-confirm_delete = Ali res želiš izbrisati "{ $world_name }"?
menu-singleplayer-confirm_regenerate = Ali res želiš ponovno generirati "{ $world_name }"?
main-login-server_not_found = Ne najdem strežnika.
main-login-authentication_error = Overitvena napaka na strežniku.
main-login-internal_error = Napaka na odjemalcu. Morda je bil igralčev lik izbrisan.
main-login-failed_auth_server_url_invalid = Ni se bilo mogoče povezati na overitveni strežnik.
main-login-insecure_auth_scheme = Overitvena shema HTTP ni podprta, ker ni varna! Za potrebe razvoja igre je HTTP dovoljen za 'localhost' in razhroščevalske različice igre.
main-login-server_full = Strežnik je poln.
main-login-untrusted_auth_server = Overitvenemu strežniku ne zaupam.
main-login-timeout = Čas je pretekel - strežnik se ni pravočasno odzval. Strežnik je morda preobremenjen ali pa so težave z omrežjem.
main-login-server_shut_down = Strežnik se je izklopil.
main-login-network_error = Omrežna napaka.
main-login-network_wrong_version = Različici strežnika in odjemalca se ne ujemata. Morda bo treba posodobiti igro.
main-login-failed_sending_request = Poizvedba na overitveni strežnik ni uspela.
main-login-invalid_character = Izbrani lik ni veljaven.
main-login-client_crashed = Odjemalec se je sesul.
main-login-not_on_whitelist = Nisi na seznamu dovoljenih uporabnikov na strežniku, na katerega se poskušaš povezati.
main-login-banned = Izključili so te iz naslednjega razloga: { $reason }
main-login-banned_until =
    Začasno so te izključili iz naslednjega razloga: { $reason }
    Izključitev traja do: { $end_date }
main-login-kicked = Dol so te vrgli iz naslednjega razloga: { $reason }
main-login-select_language = Izberi jezik
main-login-client_version = Različica odjemalca
main-login-server_version = Različica strežnika
main-login-client_init_failed = Odjemalca ni bilo mogoče zagnati: { $init_fail_reason }
main-login-username_bad_characters = Uporabniško ime vsebuje neveljavne znake! (Dovoljene so samo črke, številke, '_' in '-'.)
main-login-username_too_long = Uporabniško ime je predolgo! Največja dovoljena dolžina je { $max_len }
main-servers-select_server = Izberi strežnik
main-servers-singleplayer_error = Ni se bilo mogoče povezati na notranji strežnik: { $sp_error }
main-servers-network_error = Napaka na strežniškem omrežju/vtičnici: { $raw_error }
main-servers-participant_error = Napaka prekinitve povezave/protokola na strani sodelujočega: { $raw_error }
main-servers-stream_error = Napaka povezave/stiskanja/(de)serializacije na strani odjemalca: { $raw_error }
main-servers-database_error = Napaka strežniške podatkovne baze: { $raw_error }
main-servers-persistence_error = Napaka obstojnosti na strežniku (najbrž v povezavi s sredstvi/liki): { $raw_error }
main-servers-other_error = Splošna napaka strežnika: { $raw_error }
main-server-rules = Ta strežnik ima pravila, ki jih moraš sprejeti.
main-server-rules-seen-before = Ta pravila so se spremenila, odkar si jih zadnjič sprejel_a.
main-credits = Zasluge
main-credits-created_by = , avtor je
main-credits-music = Glasba
main-credits-fonts = Pisave
main-credits-other_art = Druga umetniška dela
main-credits-contributors = Prispevali so
loading-tips =
    .a0 = Svojo svetilko lahko prižgeš s pritiskom na '{ $gameinput-togglelantern }'.
    .a1 = Če pritisneš '{ $gameinput-controls }', si lahko ogledaš, kaj privzeto počne kakšna tipka.
    .a2 = Če hočeš govoriti z igralci, ki so v tvoji neposredni bližini, vtipkaj /say ali /s.
    .a3 = Če hočeš govoriti z igralci, ki so od tebe oddaljeni največ nekaj sto kock, vtipkaj /region ali /r.
    .a4 = Skrbniki lahko z ukazom /build preklopijo v gradbeni način.
    .a5 = Če hočeš govoriti z igralci v svoji trenutni skupini, vtipkaj /group ali /g.
    .a6 = Če hočeš nekomu poslati zasebno sporočilo, vtipkaj /tell, ime igralca in svoje sporočilo.
    .a7 = Vsepovsod po svetu se skrivajo živež, skrinje in drugi zakladi. Bodi pozoren_a nanje!
    .a8 = Je tvoj inventar poln hrane? Poskusi iz nje sestaviti boljšo hrano!
    .a9 = Iščeš pustolovščino? Podaj se v katero od temnic, ki so označene na zemljevidu!
    .a10 = Ne pozabi prilagoditi grafičnih nastavitev svojemu sistemu. Nastavitve odpreš s pritiskom na '{ $gameinput-settings }'.
    .a11 = Igranje z drugimi je zabavno! S pritiskom na '{ $gameinput-social }' si lahko ogledaš, kdo je povezan v igro.
    .a12 = S pritiskom na '{ $gameinput-dance }' lahko zaplešeš. Žurka!
    .a13 = S pritiskom na '{ $gameinput-glide }' razpri svojega zmaja in poleti v nebo.
    .a14 = Veloren je še vedno v zgodnjem razvoju. Vsak dan ga izboljšujemo!
    .a15 = Če se želiš pridružiti naši razvijalski ekipi ali pa bi rad_a le poklepetal_a z nami, pridi na naš Discord strežnik.
    .a16 = V nastavitvah lahko spremeniš način prikaza svoje življenjske energije.
    .a17 = Ob tabornem ognju lahko počakaš, da se ti rane zacelijo (k ognju se usedeš s pritiskom na '{ $gameinput-sit }').
    .a18 = Ali potrebuješ večjo torbo ali boljši oklep? S pritiskom na '{ $gameinput-crafting }' odpreš sestavljalni meni!
    .a19 = S pritiskom na '{ $gameinput-roll }' narediš preval. Prevali ti omogočajo hitrejše premikanje in izmikanje sovražnikovim napadom.
    .a20 = Se morda sprašuješ, kaj lahko sestaviš z določenim predmetom? V meni za sestavljanje vpiši poizvedbo 'input:<ime predmeta>'.
    .a21 = S pritiskom na '{ $gameinput-screenshot }' lahko zajameš posnetek zaslona.
