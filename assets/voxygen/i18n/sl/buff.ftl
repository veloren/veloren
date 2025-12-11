buff-heal = Zdravljenje
    .desc = Skozi čas ti zaceli rane.
    .stat =
        { $duration ->
            [one] Povrne { $str_total } življenjskih točk v { $duration } sekundi.
           *[other] Povrne { $str_total } življenjskih točk v { $duration } sekundah.
        }
buff-potion = Napoj
    .desc = Na dušek ga izpij ...
buff-agility = Okretnost
    .desc =
        Lahko se premikaš hitreje,
        vendar utrpiš več škode in je zadaš manj.
    .stat =
        { $duration ->
            [one]
                Poveča hitrost premikanja za { $strength } %.
                Po drugi strani znatno opešajo tvoji napadi in obramba.
                Traja { $duration } sekundo.
            [two]
                Poveča hitrost premikanja za { $strength } %.
                Po drugi strani znatno opešajo tvoji napadi in obramba.
                Traja { $duration } sekundi.
            [few]
                Poveča hitrost premikanja za { $strength } %.
                Po drugi strani znatno opešajo tvoji napadi in obramba.
                Traja { $duration } sekunde.
           *[other]
                Poveča hitrost premikanja za { $strength } %.
                Po drugi strani znatno opešajo tvoji napadi in obramba.
                Traja { $duration } sekund.
        }
buff-saturation = Nasičenje
    .desc = Prek živil se ti skozi čas krepi zdravje.
buff-resting_heal = Zdravilni počitek
    .desc = Počitek te vsako sekundo pozdravi za { $rate } % ŽT.
buff-energy_regen = Obnavljanje energije
    .desc = Hitrejše obnavljanje energije.
    .stat =
        { $duration ->
            [one] Obnovi { $str_total } energije v obdobju { $duration } sekunde.
           *[other] Obnovi { $str_total } energije v obdobju { $duration } sekund.
        }
