main-username = Användarnamn
main-server = Server
main-password = Lösenord
main-connecting = Ansluter
main-creating_world = Skapar värld
main-tip = Tips:
main-unbound_key_tip = obunden
main-notice =
    Välkommen till alfa-versionen av Veloren!
    
    Innan du dyker in i det roliga, ber vi dig hålla några saker i åtanke:
    
    - Detta är en väldigt tidig alfa-version. Förvänta dig buggar, väldigt bristfällig spelupplevelse, ovårdad spelmekanik och saknade funktioner.
    
    - Om du har konstruktiva synpunkter eller buggar att rapportera går det bra att kontakta oss på Reddit, GitLab eller vår gemenskaps Discord-server.
    
    - Veloren har öppen källkod, publicerad under licensen GPL 3. Det innebär att du får spela, modifiera och sprida spelet vidare precis
     som du vill (så länge dina ändringar publiceras under samma licens).
    
    - Veloren är ett projekt som drivs av en gemenskap utan vinstintresse och alla som bidrar är en volontär.
    Om du gillar det du ser, får du gärna gå med i en utvecklings- eller konstgrupp!
    
    Tack för att du tog dig tid att läsa det här meddelandet, vi hoppas du kommer gilla spelet!
    
    ~ Velorens utvecklare
main-login_process =
    Information om inloggningsproceduren:
    
    Observera att du behöver ett konto
    för att spela på vissa servrar.
    
    Du kan skapa ett konto på
    
    https://veloren.net/account/.
main-login-server_not_found = Servern hittades inte
main-login-authentication_error = Inloggningsfel på servern
main-login-internal_error = Internt fel hos klienten (troligen på grund av att rollpersonen har raderats)
main-login-failed_auth_server_url_invalid = Kunde inte ansluta till inloggningsservern
main-login-insecure_auth_scheme = Inloggning kan inte ske över HTTP. Detta är osäkert! För utvecklingsskäl får HTTP användas för 'localhost' ooch felsökningsinstallationer
main-login-server_full = Servern är full
main-login-untrusted_auth_server = Inloggningsservern är inte betrodd
main-login-outdated_client_or_server = ServernÄrTokig: Versionerna är förmodligen inte kompatibla, se om det finns uppdateringar tillgängliga.
main-login-timeout = Timeout: Servern svarade inte i tid. (Överbelastad eller nätverksproblem.)
main-login-server_shut_down = Servern stannade
main-login-network_error = Nätverksfel
main-login-network_wrong_version = Olika version hos server och klient, vänligen uppdatera din spelklient.
main-login-failed_sending_request = Förfrågan till inloggningsservern misslyckades
main-login-invalid_character = Den valda rollpersonen är ogiltig
main-login-client_crashed = Klienten kraschade
main-login-not_on_whitelist = Du måste finnas på administratörens vitlista för att få gå med
main-login-banned = Du har blockerats med följande motivering
main-login-kicked = Du har sparkats ut med följande motivering
main-login-select_language = Välj ett språk
main-login-client_version = Klientversion
main-login-server_version = Serverversion
main-login-client_init_failed = Klienten misslyckades att initiera: { $init_fail_reason }
main-login-username_bad_characters = Användarnamnet innehåller otillåtna tecken! (Endast alfanumeriska tecken, '_' och '-' är tillåtna)
main-login-username_too_long = Användarnamnet är för långt! Den maximala längden är: { $max_len }
main-servers-select_server = Välj en server
main-servers-singleplayer_error = Misslyckades att ansluta till den interna servern: { $sp_error }
main-servers-network_error = Serverns nätverks/uttagsfel: { $raw_error }
main-servers-participant_error = Deltagarfrånkoppling/protokollfel : { $raw_error }
main-servers-stream_error = Klientanslutnings-/komprimerings-/(av)serialiseringsfel: { $raw_error }
main-servers-database_error = Serverdatabasfel: { $raw_error }
main-servers-persistence_error = Serverpersistensfel (Troligtvis Tillgångs/Karaktärsdatarelaterat): { $raw_error }
main-servers-other_error = Generellt serverfel: { $raw_error }
main-credits = Lista över medverkande
main-credits-created_by = skapat av
main-credits-music = Musik
main-credits-fonts = Typsnitt
main-credits-other_art = Annan konst
main-credits-contributors = Bidragare
loading-tips =
    .a0 = Tryck '{ $gameinput-togglelantern }' för att tända din lykta.
    .a1 = Tryck '{ $gameinput-help }' för att se alla standardgenvägar.
    .a2 = Du kan skriva /say eller /s för att endast prata med spelare i din närhet.
    .a3 = Du kan skriva /region eller /r to för att endast prata med spelare upp till hundra block bort.
    .a4 = Administratörer kan använda kommandot /build för att växla till byggläget.
    .a5 = Du kan skriva /group eller /g för att endast prata med spelare i din nuvarande grupp.
    .a6 = Använd /tell följt av ett spelarnamn och meddelande för att kommunicera direkt med en spelare.
    .a7 = Håll ett öga öppet för mat, kistor och andra fynd som finns utspridda över hela världen!
    .a8 = Är dina väskor fyllda med mat? Testa att tillverka bättre mat från den!
    .a9 = Undrar du vad det finns att göra? Testa på en av dungeonsarna markerade på kartan!
    .a10 = Glöm inte anpassa grafiken för din dator. Tryck '{ $gameinput-settings }' för att öppna inställningarna.
    .a11 = Delad glädje är dubbel glädje! Tryck '{ $gameinput-social }' för att se vilka som spelar just nu.
    .a12 = Tryck '{ $gameinput-dance }' för att dansa. Party!
    .a13 = Tryck '{ $gameinput-glide }' för att använda din glidare och bli himlarnas härskare.
    .a14 = Veloren är fortfarande i Pre-Alpha-stadiet. Vi gör vårt yttersta för att förbättra spelet varje dag!
    .a15 = Om du vill gå med i utvecklargruppen eller bara snacka med oss får du gärna logga in på vår Discord-server.
    .a16 = Du kan välja att visa din hälsostatus i inställningarna.
    .a17 = Sitt nära en lägereld (tryck '{ $gameinput-sit }') för att långsamt återhämta dig från skador.
    .a18 = Behöver du fler väskor eller bättre rustning för din fortsatta färd? Tryck '{ $gameinput-crafting }' för att öppna tillverkningsmenyn!
    .a19 = Tryck '{ $gameinput-roll }' för att rulla. Det är användbart att rulla för att röra sig fortare och undvika fiendernas attacker.
    .a20 = Undrar du vad et föremål används till? Sök efter 'input:<item name>' i tillverkningmenyn för att se vilka recept det används i.
    .a21 = Har du hittat något coolt? Ta en bild av det med '{ $gameinput-screenshot }'.