command-ban-desc = Bannir un utilisateur en spécifiant son nom d'utilisateur pour une période donnée (si renseigné). Indiquez "true" pour modifier un bannissement déjà existant.
command-region-desc = Envoyer un message à tous le monde dans votre région ou dans le monde
command-buff-desc = Appliquer un bonus au joueur
command-dismount-desc = Démontez si vous êtes à cheval, ou faites descendre tout ce qui vous monte
command-group_invite-desc = Inviter un joueur à rejoindre votre groupe
command-jump-desc = Modifiez votre position actuelle par un décalage
command-revoke_build-desc = Révoque la permission de construire pour le joueur
command-help-template = { $usage } { $description }
command-help-list =
    { $client-commands }
    { $server-commands }

    En complément, vous pouvez utiliser les raccourcis suivants :
    { $additional-shortcuts }
command-ban-ip-desc = Bannir un utilisateur en spécifiant son nom d'utilisateur pour une période donnée (si renseigné). Contrairement au bannissement standard, l'adresse IP du joueur est également bannie. Indiquez "true" pour modifier un bannissement déjà existant.
command-battlemode_force-desc = Changez votre mode de combat sans aucune vérification
command-debug_column-desc = Affiche des informations de débogage relatives à une colonne
command-destroy_tethers-desc = Détruisez tous les liens attachés à vous
command-lightning-desc = Faire s’abattre la foudre sur votre position
command-make_npc-desc =
    Faire apparaître proche de vous une entité configurée
    Pour un exemple ou pour auto-compléter, utilisez la touche Tab.
command-adminify-desc = Octroi temporairement un rôle restreint d'administration ou retire l'actuel (si non donné)
command-alias-desc = Changer votre alias
command-area_add-desc = Ajoute une nouvelle zone de construction
command-area_list-desc = Liste toutes les zones de construction
command-area_remove-desc = Supprime une zone spécifique de construction
command-aura-desc = Créer une aura
command-body-desc = Changer votre corps pour celui d'une autre race
command-build-desc = Activer ou désactiver le mode construction
command-battlemode-desc =
    Réglez votre mode de combat sur :
    + pvp (joueur contre joueur)
    + pve (Joueur contre environnement).
    Si utilisé sans arguments, affiche le mode actuel.
command-clear_persisted_terrain-desc = Efface le terrain persistant à proximité
command-create_location-desc = Créer un emplacement à la position actuelle
command-death_effect-dest = Ajoute un effet à la mort de l'entité ciblée
command-airship-desc = Faire apparaître un dirigeable
command-campfire-desc = Faire apparaître un feu de camp
command-debug_ways-desc = Affiche des informations de débogage relatives aux chemins d’une colonne
command-delete_location-desc = Supprimer un emplacement
command-disconnect_all_players-desc = Déconnecter tous les joueurs du serveur
command-dropall-desc = Déposer tous vos objets sur le sol
command-dummy-desc = Fait apparaître un mannequin d’entraînement
command-explosion-desc = Explose le sol autour de vous
command-faction-desc = Envoyer un message à votre faction
command-goto-rand = Se téléporter à une position aléatoire
command-group-desc = Envoyer un message à votre groupe
command-group_kick-desc = Retirer un joueur de votre groupe
command-group_leave-desc = Quitter votre groupe actuel
command-group_promote-desc = Promouvoir un joueur chef de groupe
command-health-desc = Définir votre santé actuelle
command-into_npc-desc = Vous convertir en PNJ. Soyez prudent !
command-join_faction-desc = Rejoindre / quitter la faction spécifiée
command-kick-desc = Exclu le joueur avec le pseudonyme donné
command-kill-desc = Se suicider
command-kill_npcs-desc = Tuer les PNJ
command-kit-desc = Ajouter un lot d'objets à votre inventaire.
command-lantern-desc = Changer l'intensité et la couleur de votre lanterne
command-light-desc = Générer une entité lumineuse
command-location-desc = Se téléporter à l'emplacement
command-goto-desc = Se téléporter à la position
command-make_block-desc = Créer un bloc coloré à votre emplacement
command-give_item-desc = Donnez-vous des objets. Pour un exemple ou pour auto-compléter, utilisez la touche Tab.
command-make_volume-desc = Créer un volume (expérimental)
command-motd-desc = Afficher la description du serveur
command-mount-desc = Monter une entité
command-outcome-desc = Créer un revenu
command-players-desc = Lister les joueurs actuellement en ligne
command-portal-desc = Faire apparaître un portail
command-reload_chunks-desc = Recharger les chunks chargés sur le serveur
command-remove_lights-desc = Retirer toutes les lumières posées par les joueurs
command-repair_equipment-desc = Réparer tous les équipements équipés
command-reset_recipes-desc = Réinitialiser votre livre de recettes
command-respawn-desc = Vous téléporte à votre Repère
command-revoke_build_all-desc = Révoque toutes les permission de construire pour le joueur
command-safezone-desc = Créer une zone sans danger
command-object-desc = Faire apparaître un objet
command-set_body_type-not_found =
    Ce type de corps est invalide.
    Merci d'essayer avec un des suivants :
    { $options }
command-set_body_type-no_body = Impossible d'appliquer le type de corps : la cible n'a pas de corps.
command-scale-desc = Changer la taille de votre personnage
command-set_motd-desc = Changer la description du serveur
command-site-desc = Se téléporter à un site
command-skill_preset-desc = Donne les compétences désirées à votre personnage.
command-spawn-desc = Faire apparaître une entité de test
command-tell-desc = Envoyer un message à un autre joueur
command-set_body_type-not_character = Il est uniquement possible de modifier le type de corps de façon permanente si la cible est un joueur connecté en tant que personnage.
command-world-desc = Envoyer des messages à tout le monde sur le serveur
players-list-header =
    { $count ->
        [1]
            { $count } joueur en ligne
            { $player_list }
       *[other]
            { $count } joueurs en ligne
            { $player_list }
    }
command-say-desc = Envoyer un message à toutes les personnes à portée de cri
command-skill_point-desc = S'octroyer des points de compétence pour un arbre spécifique
command-set_body_type-desc = Choisissez votre type de corps, Féminin ou Masculin.
command-weather_zone-desc = Créer une zone météorologique
command-make_sprite-desc = Crée un Sprite à votre position, pour définir ses attributs utilisez la syntaxe ron pour un StructureSprite.
command-permit_build-desc = Octroie au joueur une zone délimitée dans laquelle il est possible de construire
command-ship-desc = Fait apparaître un navire
command-whitelist-desc = Ajouter / Retirer un nom d'utilisateur de la whitelist
command-set-waypoint-desc = Définissez un Repère à votre position actuelle.
command-tp-desc = Se téléporter à une autre entité
command-waypoint-desc = Affiche l'emplacement du Repère actuel
command-set-waypoint-result = Repère défini !
command-rtsim_chunk-desc = Afficher les informations à propos du chunk actuel depuis rtsim
command-group-join = Veuillez d'abord créer un groupe
command-respawn-no-waypoint = Pas de Repère défini
command-waypoint-result = Votre Repère actuel se trouve à { $waypoint } ;
command-chunk-not-loaded = Le chunk en { $x }, { $y } n'est pas chargé
command-chunk-out-of-bounds = Le chunk en { $x }, { $y } n'est pas dans les limite de la carte
command-reloaded-chunks = { $reloaded } chunks rechargés
command-set-build-mode-on-unpersistent = Mode de construction activé. Les modifications ne seront pas conservées lors du déchargement d'un chunk.
command-rtsim_purge-desc = Vider les données rtsim au prochain démarrage
command-version-desc = Affiche la version du serveur
command-wiki-desc = Ouvre le wiki ou cherche un sujet
command-clear-desc = Efface tous les messages dans le tchat. Affecte tous les onglets.
command-help-desc = Affiche des informations sur les commandes
command-preprocess-no-player-entity = Aucune entité joueur
command-mute-cannot-mute-self = Vous ne pouvez pas vous bloquer vous-même
command-mute-success = Joueur { $player } bloqué avec succès
command-mute-no-player-found = Aucun joueur trouvé avec le nom { $player }
command-mute-no-player-specified = Vous devez spécifier un joueur
command-unmute-no-muted-player-found = impossible de trouver un joueur muet avec le nom { $player }
command-unmute-no-player-specified = Vous devez spécifier un joueur à rendre muet
command-mute-already-muted = { $player } est déjà bloqué(e)
command-unmute-cannot-unmute-self = Vous ne pouvez pas vous débloquer
command-unmute-success = { $player } débloqué(e) avec succès
command-experimental-shaders-not-found = Il n'y a pas de shaders expérimentaux
command-experimental-shaders-enabled = { $shader } activé
command-experimental-shaders-disabled = { $shader } désactivé
command-experimental-shaders-not-valid = Vous devez spécifier un shader expérimental, pour avoir la liste des shaders expérimentaux, utilisez cette commande sans argument.
command-no-permission = Vous n'avez pas la permission d'utiliser '/{ $command_name }'
command-position-unavailable = Impossible d'obtenir la position de { $target }
command-experimental_shader-desc = Active/Désactive un shader expérimental.
command-preprocess-not-looking-at-valid-target = Vous ne regardez pas une cible valide
command-preprocess-not-selected-valid-target = Vous n'avez pas sélectionné une cible valide
command-preprocess-not-riding-valid-entity = Ne chevauche pas une entité valide
command-message-group-missing =
    Vous utilisez le tchat de groupe mais vous n'appartenez à aucun groupe.
    Utilisez /world ou /region pour changer de tchat.
command-mute-desc = Met en sourdine tous les messages de tchat d'un joueur.
command-rtsim_info-desc = Afficher les informations à propos d'un PNJ rtsim
command-rtsim_tp-desc = Se téléporter à un pnj rtsim
command-player-not-found = Le joueur '{ $player }' n'a pas été trouvé !
command-player-uuid-not-found = Le joueur avec l'UUID '{ $uuid }' n'a pas été trouvé !
command-username-uuid-unavailable = Impossible de déterminer l'UUID pour le nom d'utilisateur { $username }
command-uuid-username-unavailable = Impossible de déterminer le nom d'utilisateur pour l'UUID  { $uuid }
command-sudo-desc = Exécutez la commande comme si vous étiez une autre entité
command-tether-desc = Attache une entité à vous-même
command-time-desc = Réglez l'heure
command-uid-unavailable = Impossible d'obtenir l'UID pour { $target }
command-no-sudo = Ce n'est pas très poli d'usurper l'identité d'autrui
command-entity-dead = L'entité '{ $entity }' est morte !
command-error-write-settings =
    Échec de l'écriture du fichier de paramètres sur le disque, mais réussie en mémoire.
    Erreur (stockage) : { $error }
    Réussite (mémoire) : { $message }
command-error-while-evaluating-request = Une erreur s'est produite lors de la validation de la demande : { $error }
command-give-inventory-success = Ajout de { $total } x { $item } dans l'inventaire.
command-invalid-item = Objet invalide : { $item }
command-invalid-block-kind = Type de bloc invalide : { $kind }
command-nof-entities-at-least = Le nombre d'entités doit être au minimum de 1
command-nof-entities-less-than = Le nombre d'entités doit être inférieur à 50
command-entity-load-failed = Échec du chargement de la configuration de l'entité : { $config }
command-invalid-sprite = Type de sprite invalide : { $kind }
command-time-parse-too-large = { $n } est invalide : ne peut pas dépasser 16 chiffres.
command-time-parse-negative = { $n } est invalide : ne peut pas être négatif.
command-time-backwards = { $t } est avant l'heure actuelle, on ne peut pas voyager dans le passé.
command-time-invalid = { $t } n'est pas une heure valide.
command-time-current = Il est { $t }
command-rtsim-purge-perms = Vous devez être un vrai admin (pas juste un admin temporaire) pour purger les données rtsim.
command-spawned-dummy = Mannequin d'entraînement créé
command-spawned-airship = Dirigeable créé
command-spawned-campfire = Feu de camp créé
command-spawned-safezone = Zone sans danger créée
command-time-unknown = Heure inconnue
command-volume-size-incorrect = La taille doit être comprise entre 1 et 127.
command-volume-created = Volume créé
command-permit-build-given = Vous n'avez pas l'autorisation de construire dans '{ $area }'
command-permit-build-granted = Permission de construire dans '{ $area }' accordée
command-revoke-build-recv = Votre permission de construire dans '{ $area }' a été révoquée
command-revoke-build = Permission de construire dans '{ $area }' révoquée
command-revoke-build-all = Votre permission de construire à été révoquée.
command-revoked-all-build = Toutes les permissions de construire ont été révoquées.
command-no-buid-perms = Vous n'avez pas la permission de construire.
command-set-build-mode-off = Mode construction désactivé.
command-set-build-mode-on-persistent = Mode construction activé. La persistance expérimentale du terrain est activée. Le serveur tentera de conserver les modifications, mais cela n'est pas garanti.
command-set_motd-message-added = Message du jour du serveur définit comme suit : { $message }
command-set_motd-message-removed = Message du jour du serveur supprimé
command-invalid-alignment = Alignement invalide : { $alignment }
command-kit-not-enough-slots = L'inventaire n'a pas assez d'espace
command-lantern-unequiped = Veuillez d'abord équiper une lanterne
command-lantern-adjusted-strength = Puissance de la flamme ajustée.
command-lantern-adjusted-strength-color = Puissance et couleur de la flamme ajustées.
command-explosion-power-too-high = La puissance de l'explosion ne doit pas dépasser { $power }
command-explosion-power-too-low = La puissance de l'explosion doit être supérieure à { $power }
command-disconnectall-confirm =
    Veuillez exécuter la commande de nouveau avec comme deuxième argument "confirm"
    pour confirmer que vous voulez réellement déconnecter tous les joueurs du serveur
command-invalid-skill-group = { $group } n'est pas un groupe de compétences !
command-unknown = Commande inconnue
command-disabled-by-settings = Commande désactivée par les paramètres du serveur
command-battlemode-intown = Vous devez être dans une ville pour changer le mode de combat !
command-battlemode-cooldown = Cette commande ne peut pas encore être utilisée. Veuillez réessayer dans { $cooldown } secondes
command-battlemode-same = Tentative de changer vers le même mode de combat que celui actuel
command-skillpreset-broken = Présélection de compétence erronée
command-battlemode-updated = Nouveau mode de combat : { $battlemode }
command-buff-unknown = Effet inconnu : { $buff }
command-buff-data = L'argument d'effet '{ $buff }' nécessite des données supplémentaires
command-player-role-unavailable = Impossible d'obtenir les rôles administrateur pour { $target }
command-weather-valid-values = Les valeurs possibles sont 'clear' (clair), 'rain' (pluie), 'wind' (vent) et 'storm' (orage).
command-skillpreset-load-error = Erreur lors du chargement des présélections
command-skillpreset-missing = Présélection '{ $preset }' n'existe pas
command-repaired-inventory_items = Tous les objets ont été réparés
command-repaired-items = Tout les objets équipés ont été réparés
command-tell-to-yourself = Vous ne pouvez pas vous /tell vous-même.
command-adminify-assign-higher-than-own = Vous ne pouvez pas attribuer à quelqu'un un rôle temporaire supérieur à votre propre rôle permanent.
command-adminify-reassign-to-above = Vous ne pouvez pas changer le rôle d'une personne ayant un rôle équivalent ou supérieur au vôtre.
command-adminify-cannot-find-player = Impossible de trouver l'entité du joueur !
command-adminify-already-has-role = Ce joueur à déjà ce rôle !
command-adminify-already-has-no-role = Ce joueur n'a déjà plus aucun rôle !
command-adminify-role-downgraded = Rôle du joueur { $player } rétrogradé à { $role }
command-adminify-role-upgraded = Rôle du joueur { $player } monté à { $role }
command-adminify-removed-role = Rôle supprimé du joueur { $player } : { $role }
command-ban-added = { $player } ajouté(e) à la liste de bannissement pour la raison suivante : { $reason }
command-ban-already-added = { $player } est déjà sur la liste de bannissement
command-ban-ip-added = { $player } ajouté(e) à la liste de bannissement standard et la liste de bannissement par IP pour la raison suivante : { $reason }
command-ban-ip-queued = { $player } ajouté(e) à la liste de bannissement standard et mis en file d'attente pour la liste de bannissement par IP pour la raison suivante : { $reason }
command-faction-join = Veuillez rejoindre une faction avec /join_faction
command-into_npc-warning = J'espère que vous n'êtes pas en train d'en abuser !
command-kick-higher-role = Vous ne pouvez pas exclure les joueurs ayant un rôle supérieur au vôtre.
command-unban-successful = { $player } a été retiré(e) de la liste de bannissement avec succès.
command-group_invite-invited-to-your-group = Une invitation à rejoindre votre groupe a été envoyée à { $player }.
command-sudo-higher-role = Impossible de sudo des joueurs avec des rôles supérieurs au vôtre.
command-sudo-no-permission-for-non-players = Vous n'avez pas la permission de sudo des non-joueurs.
command-unban-ip-successful = L'IP bannie via l'utilisateur "{ $player }" a été retirée de la liste de bannissement par IP avec succès (cet utilisateur restera banni)
command-unban-already-unbanned = { $player } n'était déjà plus banni(e).
command-whitelist-added = Ajouté à la whitelist : { $username }
command-whitelist-already-added = Déjà dans la whitelist : { $username } !
command-whitelist-removed = Retiré de la whitelist : { $username }
command-whitelist-unlisted = Ne fait pas partie de la whitelist : { $username }
command-whitelist-permission-denied = Autorisation refusée pour supprimer l'utilisateur : { $username }
command-death_effect-unknown = Effet de mort inconnu : { $effect }.
command-cannot-send-message-hidden = Impossible d'envoyer des messages en tant que spectateur caché.
command-destroyed-tethers = Toutes les attaches ont été détruites ! Vous êtes désormais libre
command-destroyed-no-tethers = Vous n'êtes connecté(e) à aucune attache
command-no-dismount = Vous n'êtes pas sur une monture ou n'êtes pas une monture
command-player-info-unavailable = Impossible d'obtenir les informations sur le joueur { $target }
command-kit-inventory-unavailable = Impossible d'obtenir l'inventaire
command-you-dont-exist = Vous n'existez pas, vous ne pouvez donc pas utiliser cette commande
command-unban-desc = Retirer le bannissement pour le nom d'utilisateur renseigné. Si un bannissement par IP y est lié, celui-ci sera également levé.
command-unban-ip-desc = Retirer uniquement le bannissement par IP pour le nom d'utilisateur renseigné.
command-reset_tutorial-desc = Réinitialiser le tutoriel en jeu à son état de départ
command-reset_tutorial-success = Réinitialiser l'état du tutoriel.
command-preprocess-not-valid-rider = Pas de cavalier valide
command-invalid-command-message =
    Commande introuvable : { $invalid-command }.
    Vouliez-vous utiliser une des commandes suivantes ?
    { $most-similar-command }
    { $commands-with-same-prefix }

    Utilisez /help pour obtenir une liste complète des commandes.
command-experimental-shaders-list = { $shader-list }
command-experimental-shaders-not-a-shader = { $shader } n'est pas un shader expérimental, utilisez cette commande avec n'importe quel argument pour obtenir une liste complète.
command-area-not-found = La zone '{ $area }' n'a pas pu être trouvée
command-set_motd-message-not-set = Cette langue n'a pas de message du jour
command-battlemode-available-modes = Modes disponibles : jcj, jce
command-poise-desc = Changer votre équilibre actuel
command-experimental-terrain-persistence-disabled = La persistence expérimentale du terrain est désactivée
command-server-no-experimental-terrain-persistence = Le server a été compilé sans avoir activé la persistence du terrain
command-scale-set = Échelle définie à { $scale }
command-aura-spawn-new-entity = Nouvelle aura créée
command-location-invalid =
    Le nom de lieu '{ $location }' est invalide. Les noms ne peuvent contenir que des
    caractères ASCII sans majuscules et des tirets soulignés
command-location-duplicate = Le lieu '{ $location }' existe déjà, envisagez de le supprimer avant
command-location-not-found = Le lieu '{ $location }' n'existe pas
command-location-created = Lieu '{ $location }' créé
command-location-deleted = Lieu '{ $location }' supprimé
command-locations-empty = Aucun lieu n'existe actuellement
command-locations-list = Lieux disponibles : { $locations }
command-group_invite-invited-to-group = { $player } a été invité(e) au groupe.
command-site-not-found = Le Site n'a pas été trouvé
command-time_scale-current = L'échelle de temps actuelle est { $scale }.
command-time_scale-changed = Échelle de temps définie à { $scale }.
command-version-current = Le serveur est en version { $version }
command-outcome-variant_expected = Variable de résultat attendue
command-outcome-expected_entity_arg = Argument entité attendu mais non renseigné
command-outcome-expected_integer = Nombre entier attendu
command-outcome-invalid_outcome = { $outcome } n'est pas un résultat valide
command-spot-spot_not_found = Aucun endroit de ce type n'a été trouvé dans ce monde.
command-spot-world_feature = La fonctionalité 'worldgen' doit être activée pour utiliser cette commande.
command-dismounted = Descente de monture terminée
command-parse-duration-error = Impossible de déterminer la durée : { $error }
command-waypoint-error = Votre Repère n'a pas pu être trouvé.
command-unimplemented-spawn-special = Créer des entités spéciales n'est pas implémenté
command-inventory-cant-fit-item = Impossible de placer l'objet dans l'inventaire
command-spot-desc = Trouve et téléporte à un lieu d'un type spécifié le plus proche.
