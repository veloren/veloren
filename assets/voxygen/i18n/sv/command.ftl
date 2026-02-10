command-adminify-desc = Ger en spelare temporärt en begränsad adminroll eller tar bort den nuvarande (om inte gett)
command-airship-desc = Spawnar ett luftskepp
command-alias-desc = Ändra ditt alias
command-area_list-desc = Visa alla byggområden
command-area_remove-desc = Tar bort specifierat byggområde
command-aura-desc = Skapa en aura
command-body-desc = Ändra din kropp till en annan art
command-set_body_type-desc = Sätt din kroppstyp, hona eller hane.
command-help-template = { $usage } { $description }
command-area_add-desc = Lägger till ett nytt byggområde
command-help-list =
    { $client-commands }
    { $server-commands }

    Dessutom kan du använda följande kommandon:
    { $additional-shortcuts }
command-set_body_type-not_found =
    Det där är inte en giltig kroppstyp.
    Prova en av:
    { $options }
command-dropall-desc = Släpper alla dina saker på marken
command-explosion-desc = Exploderar marken runt dig
command-faction-desc = Skicka meddelanden till din fraktion
command-goto-desc = Teleportera till en position
command-goto-rand = Teleportera till en slumpmässig position
command-group-desc = Skicka meddelanden till din grupp
command-group_kick-desc = Ta bort spelare från en grupp
command-group_leave-desc = Lämna den nuvarande gruppen
command-into_npc-desc = Konvertera dig själv till en NPC. Var försiktig!
command-kill-desc = Döda dig själv
command-kill_npcs-desc = Döda NPC:erna
command-shader-backend = Nuvarande shader-backend: { $shader-backend }
command-you-dont-exist = Du existerar ej, så du kan inte använda detta kommando
command-version-current = Servern kör { $version }
command-group-join = Vänligen skapa en grupp först
command-group_invite-invited-to-group = Bjöd in { $player } till gruppen.
command-unknown = Okänt kommando
command-time-current = Det är { $t }
command-time-unknown = Tid okänd
command-volume-created = Skapade en volym
command-permit-build-given = Du är nu tillåten att bygga i ”{ $area }”
command-no-permission = Du har ej tillstånd att använda ”/{ $command_name }”
command-entity-dead = Entitet ”{ $entity }” är död!
command-experimental-shaders-list = { $shader-list }
command-weather_zone-desc = Skapa en väderzon
command-site-desc = Teleportera till en plats
command-tell-desc = Skicka ett meddelande till en annan spelare
command-say-desc = Skicka meddelanden till alla inom skrikavstånd
command-make_volume-desc = Skapa en volym (experimentellt)
command-group_invite-desc = Bjud in en spelare till att gå med i en grupp
command-build-desc = Växlar byggläge mellan på och av
command-clear-desc = Rensar alla meddelanden i chatten. Påverkar alla chattflikar.
command-experimental_shader-desc = Sätter en experimentell shader på eller av.
command-help-desc = Visa information om kommandon
command-mute-desc = Tystnar chattmeddelanden från en spelare.
command-reset_tutorial-desc = Nollställer nybörjartipsen till dess startläge.
command-unmute-desc = Otystnar en spelare som tystnades med ”mute”-kommandot.
command-mute-cannot-mute-self = Du kan inte tystna dig själv
command-mute-success = Tystnade framgångsrikt { $player }
command-mute-no-player-found = Kunde ej hitta spelare kallad { $player }
command-mute-already-muted = { $player } är redan tystnad
command-mute-no-player-specified = Du måste specificera en spelare
command-unmute-success = Otystnade framgångsrikt { $player }
command-unmute-no-muted-player-found = Kunde ej hitta en tystnad spelare kallad { $player }
command-unmute-cannot-unmute-self = Du kan inte otystna dig själv
command-unmute-no-player-specified = Du måste specificera en spelare att tystna
command-wiki-desc = Öppna wikin eller sök på ett ämne
command-battlemode-desc =
    Sätt ditt stridsläge till:
    + pvp (player vs player)
    + pve (player vs environment).
    Om körd utan argument visas nuvarande stridsläge.
command-battlemode-available-modes = Tillgängliga lägen: pvp, pve
command-battlemode-same = Försökte att sätta samma stridsläge
command-kit-not-enough-slots = Packning har inte tillräckligt med plats
command-no-sudo = Det är fräckt att imitera personer
command-player-uuid-not-found = Spelare med UUID ”{ $uuid }” ej funnen!
command-player-not-found = Spelare ”{ $player }” ej funnen!
command-location-created = Skapade plats ”{ $location }”
command-location-deleted = Raderade plats ”{ $location }”
command-location-duplicate = Plats ”{ $location }” finns redan, överväg att radera den först
command-location-not-found = Plats ”{ $location }” existerar inte
command-tell-to-yourself = Du kan inte /tell dig själv.
command-buff-unknown = Okänd buff: { $buff }
command-skillpreset-missing = Förinställning existerar ej: { $preset }
command-skillpreset-load-error = Fel medan förinställningar laddades
command-skillpreset-broken = Färdighetsförinställning är trasigt
command-disabled-by-settings = Kommando är avstängt i serverinställningar
command-battlemode-updated = Nytt stridsläge: { $battlemode }
command-battlemode-intown = Du måste vara i staden för att ändra stridsläge!
command-explosion-power-too-low = Explosionskraft måste vara mer än { $power }
command-explosion-power-too-high = Explosionskraft måste vara mindre än { $power }
command-disconnectall-confirm =
    Vänligen kör kommandot igen med det andra argumentet ”confirm” för att bekräfta att
    du verkligen vill avansluta alla spelare från servern
command-set_motd-message-added = Dagens meddelande för servern satt till { $message }
command-set_motd-message-removed = Tog bort dagens meddelande för servern
command-set_motd-message-not-set = Denna språkzon har ingen dagens meddelande satt
command-revoke-build-all = Dina byggtillstånd har upphävts.
command-revoked-all-build = Alla byggtillstånd har upphävts.
command-no-buid-perms = Du har inte tillåtelse att bygga.
command-set-build-mode-off = Växlade av bygglägge.
command-set-build-mode-on-persistent = Växlade på byggläge. Experimentell terrängfortlevande är på. Servern kommer att försöka fortleva ändringar, men det är inte garanterat.
command-set-build-mode-on-unpersistent = Växlade på byggläge. Ändringar kommer inte fortleva när en chunk avladdas.
command-lantern-unequiped = Vänligen utrusta dig själv med en lykta först
command-revoke-build-recv = Ditt byggtillstånd i ”{ $area }” har upphävts
command-permit-build-granted = Tillstånd att bygga i ”{ $area }” given
command-volume-size-incorrect = Storlek måste vara mellan 1 och 127.
command-revoke-build = Tillstånd att bygga i ”{ $area }” har upphävts
command-time-invalid = { $t } är ej en giltig tid.
command-rtsim-purge-perms = Du måste vara en riktig admin (inte en temporär) för att rensa rtsim-data.
command-nof-entities-at-least = Antal entiteter borde vara minst 1
command-nof-entities-less-than = Antal entiteter borde vara mindre än 50
command-time-parse-too-large = { $n } är ogiltig, kan ej vara större än 16 siffror.
command-time-parse-negative = { $n } är ogiltig, kan ej vara negativ.
command-time-backwards = { $t } är innan nuvarande tid, kan ej gå baklänges.
command-error-write-settings =
    Misslyckades att skriva inställningsfil till disk, men lyckades i minne.
    Fel (lagring): { $error }
    Succé (minne): { $message }
command-error-while-evaluating-request = Påträffade ett fel medan förfrågan validerades: { $error }
command-give-inventory-full =
    Spelarens packning är full. Gav { $given ->
        [1] bara en
       *[other] { $given }
    } av { $total } föremål.
command-invalid-item = Ogiltigt föremål: { $item }
command-invalid-block-kind = Ogiltig blocktyp: { $kind }
command-entity-load-failed = Misslyckades att ladda entitetskonfiguration: { $config }
command-area-not-found = Kunde inte hitta område med namnet ”{ $area }”
command-uid-unavailable = Kan ej få UID för { $target }
command-username-uuid-unavailable = Kan ej fastställa UUID för användarnamn { $username }
command-uuid-username-unavailable = Kan ej fastställa användarnamn för UUID { $uuid }
command-experimental-shaders-disabled = Stängde av { $shader }
command-experimental-shaders-not-a-shader = { $shader } är inte en experimentell shader, använd detta kommando med valfria argument för en komplett lista.
command-experimental-shaders-not-valid = Du måste specifiera en giltig experimentell shader. För att få en lista av experimentella shaders, använd detta kommando utan argument.
command-position-unavailable = Kunde ej hämta position för { $target }
command-player-role-unavailable = Kan ej hämta administratörroller för { $target }
command-experimental-shaders-enabled = Satte på { $shader }
command-experimental-shaders-not-found = Det finns inga experimentella shaders
command-preprocess-no-player-entity = Ingen spelarentitet
command-invalid-command-message =
    Kunde ej hitta kommando med namnet { $invalid-command }.
    Menade du någon av följande?
    { $most-similar-command }
    { $commands-with-same-prefix }

    Skriv /help för att se en lista över alla kommandon.
