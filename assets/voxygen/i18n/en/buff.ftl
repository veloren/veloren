## Regeneration
buff-heal = Heal
    .desc = Gain health over time.
    .stat = { $duration ->
        [1] Restores { $str_total } health points over { $duration } second.
        *[other] Restores { $str_total } health points over { $duration } seconds.
    }
## Potion
buff-potion = Potion
    .desc = Drinking...
## Agility
buff-agility = Agility
    .desc =
        Your movement is faster,
        but you deal less damage and take more damage.
    .stat = { $duration ->
        [1] Increases movement speed by { $strength } %.
            In return, your attack and defense decrease drastically.
            Lasts for { $duration } second.
        *[other] Increases movement speed by { $strength } %.
                 In return, your attack and defense decrease drastically.
                 Lasts for { $duration } seconds.
    }
## Saturation
buff-saturation = Saturation
    .desc = Gain health over time from consumables.
## Campfire/Bed
buff-resting_heal = Resting Heal
    .desc = Resting heals { $rate } % HP per second.
## Energy Regen
buff-energy_regen = Energy Regeneration
    .desc = Faster energy regeneration.
    .stat = { $duration ->
        [1] Restores { $str_total } energy over { $duration } second.
        *[other] Restores { $str_total } energy over { $duration } seconds.
    }
## Combo Generation
buff-combo_generation = Combo Generation
    .desc = Generates combo over time.
    .stat = { $duration ->
        [1] Generates { $str_total } combo over { $duration } second.
        *[other] Generates { $str_total } combo over { $duration } seconds.
    }
## Health Increase
buff-increase_max_health = Increase Max Health
    .desc = Your maximum HP is increased.
    .stat = { $duration ->
        [1] Raises maximum health
            by { $strength }.
            Lasts for { $duration } second.
        *[other] Raises maximum health
                 by { $strength }.
                 Lasts for { $duration } seconds.
    }
## Energy Increase
buff-increase_max_energy = Increase Max Energy
    .desc = Your maximum energy is increased.
    .stat = { $duration ->
        [1] Raises maximum energy
            by { $strength }.
            Lasts for { $duration } second.
        *[other] Raises maximum energy
                 by { $strength }.
                 Lasts for { $duration } seconds.
    }
## Invulnerability
buff-invulnerability = Invulnerability
    .desc = You cannot be damaged by any attack.
    .stat = { $duration ->
        [1] Grants invulnerability.
            Lasts for { $duration } second.
        *[other] Grants invulnerability.
                 Lasts for { $duration } seconds.
    }
## Protection Ward
buff-protectingward = Protecting Ward
    .desc = You are protected, somewhat, from attacks.
## Frenzied
buff-frenzied = Frenzied
    .desc = You are imbued with unnatural speed and can ignore minor injuries.
## Haste
buff-hastened = Hastened
    .desc = Your movements and attacks are faster.
## Bleeding
buff-bleed = Bleeding
    .desc = Inflicts regular damage.
## Curse
buff-cursed = Cursed
    .desc = You are cursed.
## Burning
buff-burn = On Fire
    .desc = You are burning alive.
## Crippled
buff-crippled = Crippled
    .desc = Your movement is crippled as your legs are heavily injured.
## Freeze
buff-frozen = Frozen
    .desc = Your movements and attacks are slowed.
## Wet
buff-wet = Wet
    .desc = The ground rejects your feet, making it hard to stop.
## Poisoned
buff-poisoned = Poisoned
    .desc = You feel your life withering away...
## Ensnared
buff-ensnared = Ensnared
    .desc = Vines grasp at your legs, impeding your movement.
## Fortitude
buff-fortitude = Fortitude
    .desc = You can withstand staggers, and as you take more damage you stagger others more easily.
## Parried
buff-parried = Parried
    .desc = You were parried and now are slow to recover.
## Potion sickness
buff-potionsickness = Potion sickness
    .desc = Potions have less positive effect on you after recently consuming a potion.
    .stat = { $duration ->
        [1] Decreases the positive effects of
            subsequent potions by { $strength } %.
            Lasts for { $duration } second.
        *[other] Decreases the positive effects of
                 subsequent potions by { $strength } %.
                 Lasts for { $duration } seconds.
    }
## Reckless
buff-reckless = Reckless
    .desc = Your attacks are more powerful. However, you are leaving your defenses open.
## Polymorped
buff-polymorphed = Polymorphed
    .desc = Your body changes form.
## Frigid
buff-frigid = Frigid
    .desc = Freeze your foes.
## Lifesteal
buff-lifesteal = Lifesteal
    .desc = Siphon your enemies' life away.
## Salamander's Aspect
buff-salamanderaspect = Salamander's Aspect
    .desc = You cannot burn and you move fast through lava.
## Imminent Critical
buff-imminentcritical = Imminent Critical
    .desc = Your next attack will critically hit the enemy.
## Fury
buff-fury = Fury
    .desc = With your fury, your strikes generate more combo.
## Sunderer
buff-sunderer = Sunderer
    .desc = Your attacks can break through your foes' defences and refresh you with more energy.
## Defiance
buff-defiance = Defiance
    .desc = You can withstand mightier and more staggering blows and generate combo by being hit, however you are slower.
## Bloodfeast
buff-bloodfeast = Bloodfeast
    .desc = You restore life on attacks against bleeding enemies.
## Berserk
buff-berserk = Berserk
    .desc = You are in a berserking rage, causing your attacks to be more powerful and swift, and increasing your speed. However, as a result your defensive capability is less.
## Heatstroke
buff-heatstroke = Heatstroke
    .desc = You were exposed to heat and now suffer from heatstroke. Your energy reward and movement speed are cut down. Chill.
## Scornful Taunt
buff-scornfultaunt = Scornful Taunt
    .desc = You scornfully taunt your enemies, granting you bolstered fortitude and stamina. However, your death will bolster your killer.
## Rooted
buff-rooted = Rooted
    .desc = You are stuck in place and cannot move.
## Winded
buff-winded = Winded
    .desc = You can barely breathe hampering how much energy you can recover and how quickly you can move.
## Concussion
buff-concussion = Concussion
    .desc = You have been hit hard on the head and have trouble focusing, preventing you from using some of your more complex attacks.
## Staggered
buff-staggered = Staggered
    .desc = You are off balance and more susceptible to heavy attacks.
## Tenacity
buff-tenacity = Tenacity
    .desc = You are not only able to shrug off heavier attacks, they energize you as well. However you are also slower.
## Resilience
buff-resilience = Resilience
    .desc = After having just taken a debilitating attack, you become more resilient to future incapaciting effects.
## Snare Shot
buff-snaring = Snare Shot
    .desc = Your next attack will snare the target, rooting them in place.
## Owl Talon
buff-owltalon = Owl Talon
    .desc = Taking advantage of your target not knowing of your presence, your next attack will be more precise and deal more damage.
## Heavy Nock
buff-heavynock = Heavy Nock
    .desc = Nock a heavier arrow to your bow, allowing your next shot to stagger the target. The heavier arrow will have less momentum at longer ranges though.
## Heartseeker
buff-heartseeker = Heartseeker
    .desc = Your next arrow will strike your enemy as if it'd dealt a heartwound, causing a more serious wound and giving you energy.
## Util
buff-mysterious = Mysterious effect
buff-remove = Click to remove
