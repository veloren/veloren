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
command-reload_chunks-desc = Recharger les segments chargés sur le serveur
command-remove_lights-desc = Retirer toutes les lumières posées par les joueurs
command-repair_equipment-desc = Réparer tous les équipements équipés
command-reset_recipes-desc = Réinitialiser votre livre de recettes
command-respawn-desc = Téléporter à votre point de passage
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
