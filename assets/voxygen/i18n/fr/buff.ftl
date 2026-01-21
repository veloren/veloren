## Régénération

buff-heal = Soin
    .desc = Régénère progressivement des points de vie.
    .stat =
        { $duration ->
            [1] Restaure { $str_total } de points de vie en { $duration } seconde.
           *[other] Restaure { $str_total } de points de vie en { $duration } secondes.
        }

## Potion

buff-potion = Potion
    .desc = En train de boire...

## Saturation

buff-saturation = Saturation
    .desc = Régénère progressivement des points de vie grâce à la nourriture.

## Feu de camp


## Régen d'Endurance

buff-energy_regen = Régénération d'Endurance
    .desc = Régénération de l'Endurance plus rapide.
    .stat =
        { $duration ->
            [1] Restaure { $str_total } d'Endurance en { $duration } seconde.
           *[other] Restaure { $str_total } d'Endurance en { $duration } secondes.
        }

## Augmentation de Santé

buff-increase_max_health = Augmentation de Santé
    .desc = Vos point de vie maximum sont augmentés.
    .stat =
        { $duration ->
            [1]
                Augmente les points de vie max
                de { $strength }.
                Dure { $duration } seconde.
           *[other]
                Augmente les points de vie max
                de { $strength }.
                Dure { $duration } secondes.
        }

## Augmentation d'Endurance

buff-increase_max_energy = Augmentation d'Endurance
    .desc = Votre endurance maximale est augmentée
    .stat =
        { $duration ->
            [1]
                Augmente les points d'endurance max
                de { $strength }.
                Dure { $duration } seconde.
           *[other]
                Augmente les points d'endurance max
                de { $strength }.
                Dure { $duration } secondes.
        }

## Invulnérabilité

buff-invulnerability = Invulnerability
    .desc = Vous ne pouvez pas être blessé par une attaque.
    .stat =
        { $duration ->
            [1]
                Rend invincible.
                Dure { $duration } seconde.
           *[other]
                Rend invincible.
                Dure { $duration } secondes.
        }

## Aura de Protection

buff-protectingward = Aura de Protection
    .desc = Vous êtes protégé, d'une quelconque façon, des attaques ennemies.

## Frénésie

buff-frenzied = Frénétique
    .desc = Vous bénéficiez d'une vitesse surnaturelle et ignorez les blessures superficielles.

## Hâte

buff-hastened = Hâte
    .desc = Vos mouvements et vos attaques sont plus rapides.

## Saignement

buff-bleed = Saignement
    .desc = Inflige régulièrement des dommages.

## Malédiction

buff-cursed = Maudit
    .desc = Vous êtes maudit.

## Brûlure

buff-burn = En feu
    .desc = Vous êtes en train de brûler vivant.

## Estropié

buff-crippled = Estropie
    .desc = Vos mouvements sont ralentis suite à de graves blessures aux jambes.

## Gelé

buff-frozen = Glacé(e)
    .desc = Vos mouvements et attaques sont ralentis.

## Trempé

buff-wet = Trempé(e)
    .desc = Le sol rejette vos pieds, rendant le fait de s'arrêter difficile.

## Enchaîné

buff-ensnared = Piégé(e)
    .desc = Des plantes grimpantes s'attachent à vos jambes, restreignant vos mouvements.

## Fortitude

buff-fortitude = Fortitude
    .desc = Vous pouvez résister aux étourdissements, et plus vous prenez de dégâts, plus vous étourdissez les autres facilement.

## Paré

buff-parried = Paré
    .desc = Tu as été paré et tu es maintenant lent à récupérer.

## Util

buff-remove = Cliquer pour retirer
# Reckless
buff-reckless = Imprudent
    .desc = Vos attaques sont plus puissantes mais vous laissez vos défenses ouvertes.
# Potion sickness
buff-potionsickness = Mal des potions
    .desc = Les effets des potions sont moindres après en avoir consommé une récemment.
    .stat =
        { $duration ->
            [1]
                Réduit les effets de la
                prochaine potion de { $strength } %.
                Dure { $duration } seconde.
           *[other]
                Réduit les effets de la
                prochaine potion de { $strength } %.
                Dure { $duration } secondes.
        }
# Lifesteal
buff-lifesteal = Voleur de vie
    .desc = Siphonne la vie de vos ennemis.
# Polymorped
buff-polymorphed = Polymorphe
    .desc = Votre corps change de forme.
# Salamander's Aspect
buff-salamanderaspect = Allure des salamandres
    .desc = Vous ne pouvez pas brûler et avancez vite dans la lave.
# Frigid
buff-frigid = Glacé
    .desc = Gèle vos ennemis.
# Imminent Critical
buff-imminentcritical = Coup critique imminent
    .desc = Votre prochaine attaque frappera votre ennemie de manière critique.
# Bloodfeast
buff-bloodfeast = Fête du sang
    .desc = Vous regagnez de la vie lors des attaques contre les ennemis qui saignent.
# Fury
buff-fury = Fureur
    .desc = Avec votre fureur, vos coups génèrent plus de combo.
# Sunderer
buff-sunderer = Fragmentation
    .desc = Vos attaques peuvent transpercer les défenses de vos ennemis et vous redonner plus d'endurance.
# Berserk
buff-berserk = Fou furieux
    .desc = Vous êtes dans une rage furieuse, ce qui rend vos attaques plus puissantes et plus rapides. Cependant, cela rend moindre votre capacité défensive.
buff-mysterious = Effet mystérieux
# Defiance
buff-defiance = Bravoure
    .desc = Vous pouvez résister à des coups plus puissants et étourdissants et générer des combos en étant frappé mais vous êtes plus lent.
# Agility
buff-agility = Agilité
    .desc =
         Vos mouvements sont plus rapides,
        mais vous infligez moins de dégâts, et prenez plus de dégâts.
    .stat =
        { $duration ->
            [1]
                Augmente votre vitesse de déplacement de { $strength } %.
                 En contrepartie, votre puissance d'attaque et votre défense diminuent considérablement.
                 Dure { $duration } seconde.
           *[autre]
                Augmente votre vitesse de déplacement de { $strength } %.
                En contrepartie, votre puissance d'attaque et votre défense diminuent considérablement..
                Dure { $duration } secondes.
        }
# Heatstroke
buff-heatstroke = Coup de chaleur
    .desc = Vous avez été exposé à la chaleur et vous avez pris un coup de chaud. Votre récupération d'endurance et vitesse de mouvement sont réduits. Détendez vous.
# Poisoned
buff-poisoned = Empoisonné(e)
    .desc = Vous sentez votre vitalité vous échapper…
buff-rooted = Enraciné
    .desc = Vous êtes bloqué sur place et ne pouvez pas bouger.
buff-scornfultaunt = Raillerie méprisante
    .desc = Vous raillez vos ennemis avec mépris, ce qui vous confère une force d'âme et une endurance accrues. Cependant, votre mort renforcera votre tueur.
buff-staggered = Chancèlement
    .desc = Vous êtes déséquilibré et plus vulnérable aux attaques lourdes.
buff-concussion = Commotion
    .desc = Vous avez été frappé fort à la tête et n'arrivez plus à vous concentrer, vous empêchant d'utiliser certaines de vos attaques les plus complexes.
buff-resting_heal = Repos revigorant
    .desc = Se reposer soigne { $rate } % de points de vie par seconde.
buff-resilience = Résilience
    .desc = Après avoir subi une attaque débilitante, vous devenez plus résilient aux futurs effets incapacitants.
buff-winded = Manque de souffle
    .desc = Vous pouvez à peine respirer, ce qui entrave la quantité d'endurance que vous pouvez récupérer et réduit votre vitesse de déplacement.
buff-tenacity = Tenacité
    .desc = En plus d'être capable d'ignorer les attaques lourdes, elles vous donnent de l’endurance également. Cependant, vous êtes maintenant plus lent.
buff-combo_generation = Génération de combo
    .desc = Génère du combo au fil du temps.
    .stat =
        { $duration ->
            [1] Génère { $str_total } combo en { $duration } seconde.
           *[other] Génère { $str_total } combo en { $duration } secondes.
        }
buff-owltalon = Serres de chouette
    .desc = Prenant avantage de l'ignorance de votre cible quand à votre présence, votre prochaine attaque sera plus précise et infligera davantage de dommages.
buff-heavynock = Encoche lourde
    .desc = Encochez une flèche plus lourde, permettant à votre prochain tir d'étourdir sa cible. Cette flèche plus lourde aura cependant moins d'inertie à de longues distances.
buff-heartseeker = Cherchecœur
    .desc = Votre prochaine flèche frappera votre ennemi de manière similaire à une blessure au cœur, causant une blessure plus sévère et vous donnant de l'endurance.
buff-eagleeye = Œil d'aigle
    .desc = Vous pouvez observer les points vulnérables de vos cibles avec clarté, et avez l'agilité nécessaire pour guider chaque flèche sur ceux-ci.
buff-chilled = Au frais
    .desc = Le froid intense vous ralenti et vous laisse davantage vulnérable aux attaques puissantes.
buff-ardenthunter = Chasse acharnée
    .desc = Votre ferveur permet à vos flèches d'être plus léthales contre une cible spécifique, et votre endurance augmente à l'instar du nombre de flèches atteignant celle-ci.
buff-ardenthunted = Acharnement de la chasse
    .desc = Vous êtes la cible d'une chasse archarnée.
buff-septicshot = Tir septique
    .desc = Votre prochaine flèche inflige une infection à la cible, plus léthale si celle-ci subit déjà des effets négatifs.
