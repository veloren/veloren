command-no-permission = Non hai i permessi per eseguire '/{ $command_name }'
command-position-unavailable = Impossibile ottenere la posizione di { $target }
command-player-role-unavailable = Non puoi ottenere i privilegi di amministratore per { $target }
command-uid-unavailable = Impossibile ottenere l'identificativo di { $target }
command-area-not-found = Non c'è un'area chiamata '{ $area }'
command-player-not-found = Giocatore '{ $player }' non trovato!
command-player-uuid-not-found = Non trovo il giocatore con l'identificativo '{ $uuid }'!
command-username-uuid-unavailable = Non riesco a determinare l'identificativo di { $username }
command-uuid-username-unavailable = Non riesco a trovare il nome utente di { $uuid }
command-no-sudo = È scortese impersonare altri giocatori
command-entity-dead = L'entità '{ $entity }' è morta!
command-error-while-evaluating-request = Si è verificato un errore durante la convalida della richiesta: { $error }
command-give-inventory-full = L'inventario del giocatore è pieno. Cedi { $given ->
  [1] solo uno
  *[other] { $given }
} di { $total } oggetti.
command-invalid-item = Articolo non valido: { $item }
command-invalid-block-kind = Tipo di blocco non valido: { $kind }
command-nof-entities-at-least = Il numero di entità deve essere almeno 1
command-nof-entities-less-than = Il numero di entità deve essere inferiore a 50
command-entity-load-failed = Impossibile caricare la configurazione dell'entità: { $config }
command-spawned-entities-config = Generate { $n } entità dalla configurazione: { $config }
command-invalid-sprite = Tipo di sprite non valido: { $kind }
command-time-parse-too-large = { $n } non è valido, non può essere più lungo di 16 cifre
command-time-parse-negative = { $n } non è valido, non può essere negativo.
command-time-backwards = { $t } è precedente all'ora corrente, il tempo non può andare indietro.
command-time-invalid = { $t } non è un ora valida.
command-rtsim-purge-perms = Devi essere un vero amministratore (non solo un amministratore temporaneo) per eliminare i dati rtsim.
command-chunk-not-loaded = L'area { $x }, { $y } non è stato caricato
command-chunk-out-of-bounds = L'area { $x }, { $y } è esterna dalla mappa
command-spawned-entity = Hai generato una entità con ID: { $id }
command-spawned-dummy = Hai generato un manichino da allenamento
command-spawned-airship = Hai generato un dirigibile
command-spawned-campfire = Ha generato un falò
command-spawned-safezone = Ha generato una zona sicura
command-volume-size-incorrect = La dimensione deve essere compresa tra 1 e 127.
command-volume-created = Volume creato
command-permit-build-given = Non puoi costruire in '{ $area }'
command-permit-build-granted = Permesso di costruire in '{ $area }' concesso
command-revoke-build-recv = Il permesso di costruire in '{ $area }' è stato revocato
command-revoke-build = Permesso di costruire in '{ $area }' revocato
command-revoke-build-all = I tuoi permessi di costruzione sono stati revocati.
command-revoked-all-build = Tutti i permessi di costruzione sono stati revocati
command-no-buid-perms = Non hai il permesso di costruire.
command-set-build-mode-off = Disattivata la modalità di costruzione.
command-set-build-mode-on-persistent = Attivata la modalità di costruzione. La persistenza del terreno sperimentale è abilitata. Il server tenterà di rendere persistenti le modifiche, ma ciò non è garantito
command-set-build-mode-on-unpersistent = Attivata la modalità di costruzione. Le modifiche non verranno mantenute quando un blocco viene scaricato.
command-invalid-alignment = Allineamento non valido: { $alignment }
command-kit-not-enough-slots = Non c'è abbastanza spazio disponibile nell'inventario
command-lantern-unequiped = Per favore serve prima una lanterna
command-lantern-adjusted-strength = Hai modificato l'intensità della lanterna.
command-lantern-adjusted-strength-color = Hai modificato l'intensità e il colore della lanterna.
command-explosion-power-too-high = La potenza di esplosione non deve essere superiore a { $power }
command-explosion-power-too-low = La potenza di esplosione deve essere superiore a { $power }
# Note: Do not translate "confirm" here
command-disconnectall-confirm = Esegui nuovamente il comando con il secondo argomento "confirm" per confermarlo
  vuoi davvero disconnettere tutti i giocatori dal server
command-invalid-skill-group = { $group } non è un gruppo di abilità!
command-unknown = Comando sconosciuto
command-disabled-by-settings = Comando disabilitato nelle impostazioni del server
command-battlemode-intown = Devi essere in città per cambiare la modalità battaglia!
command-battlemode-cooldown = Tregua attiva. Riprova tra { $cooldown } secondi
command-battlemode-available-modes = Modalità disponibili: pvp, pve
command-battlemode-same = Tentativo di impostare la stessa modalità di battaglia
command-battlemode-updated = Nuova modalità di battaglia: { $battlemode }
command-buff-unknown = Modificatore sconosciuto: { $buff }
command-buff-data = Il modificatore '{ $buff }' richiede informazioni aggiuntive
command-buff-body-unknown = Specifiche sconosciute: { $spec }
command-skillpreset-load-error = Errore durante il caricamento delle preimpostazioni
command-skillpreset-broken = La preimpostazione dell'abilità è guasta
command-skillpreset-missing = Preimpostazione mancante: { $preset }
command-location-invalid = Il luogo '{ $location }' è invalido. I nomi possono contenere solo caratteri ASCII minuscoli e
   trattino basso
command-location-duplicate = La località '{ $location }' esiste già, valuta di cancellarla prima
command-location-not-found = La località '{ $location }' non esiste
command-location-created = Località creata '{ $location }'
command-location-deleted = Località cancellata '{ $location }'
command-locations-empty = Al momento non esiste alcuna località
command-locations-list = Località disponibili: { $locations }
# Note: Do not translate these weather names
command-weather-valid-values = I valori valido sono 'clear', 'rain', 'wind', 'storm'
command-scale-set = Imposta la scala a { $scale }
command-repaired-items = Riparato l'intero equipaggiamento
command-message-group-missing = Stai utilizzando la chat di gruppo ma non appartieni ad alcun gruppo. Usa /world o
   /region per cambiare chat.
command-tell-request = { $sender } Vuole parlare con te.

# Unreachable/untestable but added for consistency

command-player-info-unavailable = Non posso trovare le informazioni per il giocatore { $target }
command-unimplemented-waypoint-spawn = La generazione delle tappe non è implementata
command-unimplemented-teleporter-spawn = La generazione dei portali non è implementata
command-kit-inventory-unavailable = Impossibile ottenere l'inventario
command-inventory-cant-fit-item = Impossibile adattare l'articolo all'inventario
# Emitted by /disconnect_all when you dont exist (?)
command-you-dont-exist = Tu non esisti, quindi non puoi usare questo comando
command-destroyed-tethers = Distrutti tutti i legami! Ora sei libero
command-destroyed-no-tethers = Non hai nessun legame
command-dismounted = Smontato
command-no-dismount = Non stai cavalcando né sei cavalcato
