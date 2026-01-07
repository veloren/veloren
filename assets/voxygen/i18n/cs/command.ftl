command-help-template = { $usage } { $description }
command-help-list =
    { $client-commands }
    { $server-commands }

    Případně můžeš použít následující zkratky:
    { $additional-shortcuts }
command-adminify-desc = Dočasně dá hráči omezenou administrátorskou roli nebo odstraní stávající (pokud nebyla udělena)
command-airship-desc = Přidá vzducholoď
command-alias-desc = Změň si přezdívku
command-area_add-desc = Přidá novou stavební oblast
command-area_list-desc = Vypíše stavební oblasti
command-area_remove-desc = Odstraní danou stavební oblast
command-aura-desc = Vytvoří auru
command-body-desc = Změň si tělo na jiný druh
command-set_body_type-desc = Nastav si typ těla, Ženské nebo Mužské.
command-set_body_type-not_found =
    Tohle není správný typ těla.
    Zkus jedno z:
    { $options }
command-set_body_type-no_body = Nešlo nastavit typ těla, protože cíl nemá tělo.
command-set_body_type-not_character = Trvale lze změnit typ těla jen pokud je cílem hráč přihlášený s postavou.
command-buff-desc = Sešle na hráče posílení
command-build-desc = Zapne nebo vypne stavební mód
command-ban-desc = Zablokuj hráče s daným uživatelským jménem na danou dobu (pokud je dána). Pošli "true" pro přepsání nebo úpravu existujícího bloku.
command-ban-ip-desc = Zablokuj hráče s daným uživatelským jménem na daný čas (pokud je daný). Narozdíl od normálního bloku tohle navíc zablokuje IP adresu tohoto hráče. Pošli "true" k přepsání nebo upravení existujícího bloku.
command-battlemode-desc =
    Nastav si bojový mód na:
    + pvp (hráč proti hráči)
    + pve (hráč proti prostředí)
    Pokud není dán argument, ukáže aktuální bojový mód.
command-battlemode_force-desc = Změň si bojový mód bez jakýchkoliv kontrol
command-campfire-desc = Přidá táborák
command-clear_persisted_terrain-desc = Vyčistí blízký terén
command-create_location-desc = Vytvoří lokaci na aktuální pozici
command-death_effect-dest = Přidá efekt k úmrtí cílové entity
command-debug_column-desc = Vypíše debugovací informace o sloupci
command-delete_location-desc = Vymaž lokaci
command-disconnect_all_players-desc = Odpojí všechny hráče ze serveru
command-dismount-desc = Slez, pokud na něčem jedeš, nebo se zbav čehokoliv, co jede na tobě
command-dropall-desc = Odhodí všechny tvé předměty na zem
command-dummy-desc = Přidá trénovacího panáka
command-explosion-desc = Země kolem tebe vybuchne
command-faction-desc = Pošli zprávu tvému cechu
command-give_item-desc = Dej si nějaké předměty. Pro příklad nebo automatické doplnění stiskni Tab.
command-goto-desc = Teleportuj se na pozici
command-goto-rand = Teleportuj se na náhodnou pozici
command-group-desc = Pošli zprávu svojí skupině
command-group_invite-desc = Přizvi hráče, aby se přidal do skupiny
command-group_kick-desc = Odstraň hráče ze skupiny
command-group_leave-desc = Opusť současnou skupinu
command-group_promote-desc = Povyš hráče na vedoucího skupiny
command-health-desc = Nastav si životy
command-into_npc-desc = Přepni se na NPC. Opatrně!
command-join_faction-desc = Přidej se/opusť daný cech
command-jump-desc = Posuň svou současnou pozici
command-kick-desc = Vykopni hráče s daným uživatelským jménem
command-kill-desc = Zabij se
command-kill_npcs-desc = Zabij NPC
command-kit-desc = Přidej si do inventáře sadu předmětů.
command-lantern-desc = Změň sílu a barvu svojí lucerny
command-light-desc = Přidej entitu se světlem
command-lightning-desc = Sešli blesk na současnou pozici
command-location-desc = Teleportuj se na lokaci
command-make_block-desc = Vytvoř na své lokaci barevný blok
command-make_npc-desc =
    Přidej entitu z konfigurace poblíž sebe.
    Pro příklad nebo automatické doplnění stiskni Tab.
command-make_sprite-desc = Vytvoř sprite na své lokaci. K definici atributů spritu použij ron syntax pro StructureSprite.
command-make_volume-desc = Vytvoř těleso (experimentální)
command-motd-desc = Zobraz popisek serveru
command-mount-desc = Nasedni na entitu
command-object-desc = Přidej objekt
command-outcome-desc = Vytvoř výsledek
command-permit_build-desc = Dá hráči omezenou kostku, ve které může stavět
command-players-desc = Zobrazí aktuálně připojené hráče
command-poise-desc = Nastav svůj současný postoj
command-portal-desc = Přidá portál
command-region-desc = Pošle zprávu všem ve tvém regionu
command-reload_chunks-desc = Znovu načte chunky aktuálně načtené na serveru
command-remove_lights-desc = Odstraní všechna světla přidaná hráči
command-repair_equipment-desc = Opraví všechny vybavené předměty
command-reset_recipes-desc = Resetuje tvůj receptář
command-respawn-desc = Teleportovat k uloženému bodu
command-revoke_build-desc = Odebere hráči práva k stavbě
command-revoke_build_all-desc = Odebere hráči práva ke všem stavebním oblastem
command-safezone-desc = Vytvoří bezpečnou zónu
command-say-desc = Pošli zprávu všem hráčům v doslechu
command-scale-desc = Škáluj svou postavu
command-set_motd-desc = Nastav popisek serveru
command-set-waypoint-desc = Nastav si záchytný bod na aktuální lokaci.
command-ship-desc = Přidá loď
command-site-desc = Teleportuj se na místo
command-skill_point-desc = Dej si body zkušeností pro daný strom dovedností
command-skill_preset-desc = Dá tvé postavě požadované schopnosti.
command-spawn-desc = Přidej testovací entitu
command-spot-desc = Najdi a teleportuj se na nejbližší místo daného druhu.
command-sudo-desc = Spusť příkaz jako jiná entita
command-tell-desc = Pošli zprávu jinému hráči
command-tether-desc = Připoutej k sobě jinou entitu
command-time-desc = Nastav denní dobu
command-time_scale-desc = Nastav škálování časového rozdílu
command-tp-desc = Teleportuj se k jiné entitě
command-rtsim_chunk-desc = Zobraz informace o aktuálním chunku z rtsim
command-rtsim_info-desc = Zobraz informace o rtsim NPC
command-rtsim_npc-desc = Vypiš rtsim NPC, která sedí na daný filtr (jako "simulated", "merchant"), seřazené podle vzdálenosti
command-rtsim_purge-desc = Vyčisti rtsim data při dalším spuštění
command-rtsim_tp-desc = Teleportuj se k rtsim NPC
command-unban-desc = Odstraň blok daného uživatelského jména. Pokud je k němu zaznamenaný blok IP adresy, bude odstraněn i ten.
command-unban-ip-desc = Odstraň jen IP blok pro dané uživatelské jméno.
command-version-desc = Vypíše verzi serveru
command-weather_zone-desc = Vytvoř zónu s počasím
command-whitelist-desc = Přidá/odstraní uživatele z whitelistu
command-world-desc = Pošli zprávu všem na serveru
command-wiki-desc = Otevři wiki nebo hledej téma
command-reset_tutorial-desc = Resetuj tutoriál na začátek
command-reset_tutorial-success = Resetuj stav tutoriálu.
players-list-header =
    { $count ->
        [1]
            { $count } hráč připojený
            { $player_list }
        [few]
            { $count } hráči připojení
            { $player_list }
       *[other]
            { $count } hráčů připojených
            { $player_list }
    }
command-clear-desc = Vyčistí všechny zprávy v chatu. Má vliv na všechny záložky.
command-experimental_shader-desc = Zapne experimentální shadery.
command-help-desc = Zobraz informace o příkazu
command-mute-desc = Ztlumí zprávy od všech hráčů.
command-unmute-desc = Zruší 'mute' příkaz pro daného hráče.
command-waypoint-desc = Ukaž lokaci aktuálního záchytného bodu
command-preprocess-target-error = Očekáváno { $expected_list } po '@', nalezeno { $target }
command-preprocess-not-looking-at-valid-target = Nedíváš se na správný cíl
command-preprocess-not-selected-valid-target = Nevybral(a) jsi správný cíl
