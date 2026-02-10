## Regeneration

buff-heal = Cura
    .desc = Recuperi salute nel tempo.
    .stat =
        { $duration ->
            [1] Ripristina { $str_total } punti salute in { $duration } secondo.
           *[other] Ripristina { $str_total } punti salute in { $duration } secondi.
        }

## Potion

buff-potion = Pozione
    .desc = Bevendo...

## Agility

buff-agility = Agilità
    .desc = Il tuo movimento è più veloce, ma infliggi meno danni e subisci più danni.
    .stat =
        Aumenta la velocità di movimento del { $strength }%,
        ma diminuisce il danno del 100%,
        e aumenta la tua vulnerabilità ai danni
        del 100%.

## Saturation

buff-saturation = Saturazione
    .desc = Guadagna salute nel tempo dai cibi.

## Campfire


## Energy Regen

buff-energy_regen = Rigenerazione di energia
    .desc = Rigenerazione di energia più veloce
    .stat = Ripristina { $str_total } di energia

## Health Increase

buff-increase_max_health = Aumenta la salute massima
    .desc = Aumenta il limite massimo di salute
    .stat =
        Aumenta la salute massima
        di { $strength }

## Energy Increase

buff-increase_max_energy = Aumenta l'energia massima
    .desc = Aumenta il limite massimo di energia
    .stat =
        Aumenta la massima energia
        di { $strength }

## Invulnerability

buff-invulnerability = Invulnerabilità
    .desc = Non puoi essere danneggiato da nessun attacco.
    .stat = Dona invulnerabilità

## Protection Ward

buff-protectingward = Barriera protettiva
    .desc = Sei un po' protetto dagli attacchi.

## Frenzied

buff-frenzied = Frenetico
    .desc = Sei imbevuto con velocità innaturale e puoi ignorare ferite minori.

## Haste

buff-hastened = Rapido
    .desc = I tuoi movimenti e i tuoi attacchi sono più veloci.

## Bleeding

buff-bleed = Sanguinante
    .desc = Infligge danno regolare.

## Curse

buff-cursed = Maledetto
    .desc = Sei maledetto.

## Burning

buff-burn = A fuoco
    .desc = Stai bruciando vivo

## Crippled

buff-crippled = Storpio
    .desc = Il tuo movimento è storpio dal momento che le tue gambe sono gravemente ferite.

## Freeze

buff-frozen = Congelato
    .desc = I tuoi movimenti e i tuoi attacchi sono rallentati.

## Wet

buff-wet = Bagnato
    .desc = Il terreno rifiuta i tuoi piedi, rendendoti difficile fermarsi.

## Poisoned

buff-poisoned = Avvelenato
    .desc = Senti la tua vita svanire...

## Ensnared

buff-ensnared = Intrappolato
    .desc = Liane si attorcigliano intorno alle tue gambe, impedendo il tuo movimento.

## Fortitude

buff-fortitude = Forza d'animo
    .desc = Puoi resistere allo stordimento.

## Parried

buff-parried = Bloccato
    .desc = Sei stato bloccato e ora sei lento a riprenderti.

## Potion sickness

buff-potionsickness = Nausea da pozione
    .desc = Le pozioni ti cureranno meno.
    .stat =
        Le pozioni che berrai ti cureranno
        del { $strength }% in meno.

## Reckless

buff-reckless = Spericolato
    .desc = I tuoi attacchi sono più potenti, tuttavia lasci le tue difese scoperte.

## Polymorped

buff-polymorphed = Mutaforma
    .desc = Il tuo corpo cambia la forma.

## Flame


## Frigid

buff-frigid = Glaciale
    .desc = Congela i tuoi nemici.

## Lifesteal

buff-lifesteal = Rubavita
    .desc = Risucchia la vita dai tuoi nemici.

## Salamander's Aspect

buff-salamanderaspect = Aspetto di salamandra
    .desc = Non puoi bruciare e ti muovi velocemente sulla lava.

## Imminent Critical

buff-imminentcritical = Critico imminente
    .desc = Il tuo prossimo attacco colpirà l'avversario in modo critico.

## Fury

buff-fury = Furia
    .desc = Con la tua furia i tuoi colpi generano più combo.

## Sunderer

buff-sunderer = Spaccadifese
    .desc = I tuoi attacchi possono sfondare le difese dei tuoi nemici e rigenerarti con più energia.

## Sunderer

buff-defiance = Sfida
    .desc = Puoi resistere a colpi più potenti e stordenti e generare combo venendo colpito, ma sei più lento.

## Bloodfeast

buff-bloodfeast = Festa di sangue
    .desc = Ti rigeneri attaccando nemici che sanguinano.

## Berserk

buff-berserk = Frenesia
    .desc = Sei in una furia frenetica i tuoi attacchi sono più veloci e potenti, ma la tua difesa sarà inferiore.

## Heatstroke

buff-heatstroke = Colpo di calore
    .desc = Sei stato esposto al calore e ora soffri di un colpo di calore, il tuo movimento e il recupero di energia sono ridotti. Brividi.

## Util

buff-mysterious = Effetto misterioso
buff-remove = Premi per rimuovere
buff-resting_heal = Cura a Riposo
    .desc = A riposo rigeneri il { $rate } % di salute al secondo.
buff-combo_generation = Generazione di combo
    .desc = Genera combo nel tempo
    .stat =
        { $duration ->
            [1] Genera { $str_total } combo in un secondo.
           *[other] Genera { $str_total } combo in { $duration } secondi.
        }
