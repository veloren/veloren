main-username = Användarnamn
main-server = Server
main-password = Lösenord
main-connecting = Ansluter
main-creating_world = Skapar värld
main-tip = Tips:
main-unbound_key_tip = obunden
main-notice =
    Välkommen till alfaversionen av Veloren!

    Innan du dyker in i det roliga ber vi dig att hålla några saker i åtanke:

    – Detta är en väldig tidig alfaversion. Förvänta dig buggar, väldigt bristfällig spelupplevelse, ovårdad spelmekanik och saknade funktioner.

    – Om du har konstruktiva synpunkter eller buggrapporter kan du kontakta oss på vår GitLab-repo och på vår Discord- eller Matrix-server.

    – Veloren är öppen källkod. Du får spela, modifiera, och sprida spelet vidare som du vill i enlighet med version 3 av GNU General Public License.

    – Veloren är ett ideellt community-projekt, och alla som jobbar på det är en volontär.
    Om du gillar det du ser får du gärna gå med i någon av våra arbetsgrupper!

    Tack för att du tog dig tid att läsa det här meddelandet, vi hoppas du kommer gilla spelet!

    ~ Utvecklarteamet
main-login_process =
    Om multiplayer:

    Notera att du behöver ett konto för att spela på servrar med inloggning på.

    Du kan skapa ett konto på:
    https://veloren.net/account/
main-login-server_not_found = Servern hittades inte.
main-login-authentication_error = Inloggningsfel på servern.
main-login-internal_error = Internt fel hos klient. Tips: Spelarkaraktären kan ha blivit raderad.
main-login-failed_auth_server_url_invalid = Kunde inte ansluta till inloggningsservern.
main-login-insecure_auth_scheme = Inloggning stöds inte över HTTP. Det är osäkert! För utvecklingsskäl får HTTP användas för ”localhost” eller felsökningsversioner.
main-login-server_full = Servern är full.
main-login-untrusted_auth_server = Inloggningsservern är inte betrodd.
main-login-timeout = Timeout: Servern svarade inte i tid. Tips: servern kanske är överbelastad eller så är det problem i nätverket.
main-login-server_shut_down = Servern stängdes ner.
main-login-network_error = Nätverksfel.
main-login-network_wrong_version = Olika versioner hos server och klient. Tips: Du kanske behöver uppdatera din spelklient.
main-login-failed_sending_request = Förfrågan till inloggningsservern misslyckades.
main-login-invalid_character = Den valda karaktären är ogiltig.
main-login-client_crashed = Klient kraschade.
main-login-not_on_whitelist = Du är inte medlem i vitlistan på servern du försökte gå med i.
main-login-banned = Du har blivit permanent blockerad med följande anledning: { $reason }
main-login-kicked = Du har sparkats ut med följande anledning: { $reason }
main-login-select_language = Välj språk
main-login-client_version = Klientversion
main-login-server_version = Serverversion
main-login-client_init_failed = Klienten misslyckades att initiera: { $init_fail_reason }
main-login-username_bad_characters = Användarnamnet innehåller ogiltiga tecken! (Endast alfanumeriska tecken, ”_” och ”-” är tillåtna.)
main-login-username_too_long = Användarnamnet är för långt! Maxlängd är: { $max_len }
main-servers-select_server = Välj server
main-servers-singleplayer_error = Misslyckades att ansluta till den interna servern: { $sp_error }
main-servers-network_error = Serverns nätverks-/uttagsfel: { $raw_error }
main-servers-participant_error = Deltagarfrånkoppling/protokollfel : { $raw_error }
main-servers-stream_error = Klientanslutnings-/komprimerings-/(av)serialiseringsfel: { $raw_error }
main-servers-database_error = Serverdatabasfel: { $raw_error }
main-servers-persistence_error = Serverpersistensfel (Troligtvis Tillgångs/Karaktärsdatarelaterat): { $raw_error }
main-servers-other_error = Generellt serverfel: { $raw_error }
main-credits = Medverkande
main-credits-created_by = skapat av
main-credits-music = Musik
main-credits-fonts = Typsnitt
main-credits-other_art = Annan konst
main-credits-contributors = Bidragare
loading-tips =
    .a0 = Tryck ”{ $gameinput-togglelantern }” för att tända din lykta.
    .a1 = Tryck ”{ $gameinput-controls }” för att se alla standardtangentbordsgenvägar.
    .a2 = Du kan skriva /say eller /s för att bara prata med spelare i din närhet.
    .a3 = Du kan skriva /region eller /r to för att bara prata med spelare några hundra block omkring dig.
    .a4 = Administratörer kan använda kommandot /build för att växla till byggläget.
    .a5 = Du kan skriva /group eller /g för att bara prata med spelare i din nuvarande grupp.
    .a6 = För att skicka privata meddelanden använd /tell följt av ett spelarnamn och ditt meddelande.
    .a7 = Håll ett öga öppet för mat, kistor och andra fynd som finns utspridda över hela världen!
    .a8 = Är din packning fylld med mat? Testa att tillverka bättre mat från det!
    .a9 = Undrar du vad det finns att göra? Testa på en av dungeonsarna markerade på kartan!
    .a10 = Glöm inte anpassa grafiken för din dator. Tryck på ”{ $gameinput-settings }” för att öppna inställningarna.
    .a11 = Att spela med andra är kul! Tryck på ”{ $gameinput-social }” för att se vilka som är online.
    .a12 = Tryck på ”{ $gameinput-dance }” för att dansa. Party!
    .a13 = Tryck på ”{ $gameinput-glide }” för att använda din glidare och bli himlarnas härskare.
    .a14 = Veloren är fortfarande i pre-alpha. Vi gör vårt bästa för att förbättra det varje dag!
    .a15 = Om du vill gå med i utvecklargruppen eller bara snacka med oss, gå med i vår Discord-server.
    .a16 = Du kan välja att visa din hälsostatus i inställningarna.
    .a17 = Sitt nära en lägereld (med ”{ $gameinput-sit }”-knappen) för att långsamt återhämta dig från skador.
    .a18 = Behöver du fler väskor eller bättre rustning för din fortsatta färd? Tryck på ”{ $gameinput-crafting }” för att öppna tillverkningsmenyn!
    .a19 = Tryck på ”{ $gameinput-roll }” för att rulla. Rullning kan användas för att röra sig fortare och undvika fiendernas attacker.
    .a20 = Undrar du vad ett föremål används till? Sök ”input:<item name>” i tillverkningmenyn för att se vilka recept det används i.
    .a21 = Du kan ta skärmbilder med ”{ $gameinput-screenshot }”.
main-singleplayer-new = Ny
main-singleplayer-regenerate = Omgenerera
main-singleplayer-create_custom = Skapa Anpassad
main-singleplayer-size_lg = Logaritmisk storlek
main-singleplayer-random_seed = Slumpmässig
main-singleplayer-day_length = Daglängd
main-singleplayer-world_name = Världnamn
main-singleplayer-map_scale = Vertikal skalning
main-singleplayer-map_erosion_quality = Erosionskvalitet
main-singleplayer-play = Spela
main-singleplayer-generate_and_play = Generera och spela
menu-singleplayer-confirm_regenerate = Vill du verkligen omgenerera ”{ $world_name }”?
menu-singleplayer-confirm_delete = Vill du verkligen radera ”{ $world_name }”?
main-singleplayer-map_shape = Form
main-singleplayer-map_large_warning = Varning: Stora världar kommer att ta lång tid att starta den första gången.
main-server-rules-seen-before = De här reglerna har ändrats sen du sist accepterat dem.
main-singleplayer-delete = Radera
main-server-rules = Den här servern har regler som måste accepteras.
main-singleplayer-seed = Frö
main-singleplayer-map_shape-circle = Cirkel
main-singleplayer-map_shape-square = Kvadrat
main-login-banned_until =
    Du har blivit temporärt blockerad med följande anledning: { $reason }
    Fram till: { $end_date }
main-singleplayer-map_large_extra_warning = Detta skulle ta ungefär samma mängd resurser som att generera { $count } världar med standardinställningar.
