main-username = Nom d'utilisateur
main-server = Serveur
main-password = Mot de passe
main-connecting = Connexion
main-creating_world = Création du monde
main-tip = Astuce :
main-unbound_key_tip = non définie
main-notice =
    Bienvenue dans la version alpha de Veloren !

    Avant de commencer à vous amuser, merci de garder les choses suivantes en tête :

    - Le jeu est actuellement en alpha très précoce. Attendez-vous à des bugs, des boucles de gameplay non terminées, des mécaniques non peaufinées et des fonctionalités manquantes.

    - Si vous avez des retours constructifs ou avez trouvé un bug, vous pouvez nous contacter via sur notre dépôt GitLab et sur notre serveur Discord ou Matrix.

    - Veloren est un jeu open source. Vous êtes libre de jouer, de modifier et de redistribuer le jeu conformément à la version 3 de la licence publique générale GNU.

    - Veloren est un projet communautaire à but non-lucratif développé par des bénévoles.
    Si vous appréciez ce jeu, vous êtes les bienvenus pour rejoindre l'une de nos équipes !

    Merci d'avoir pris le temps de lire cette annonce, nous espérons que vous apprécierez le jeu !

    ~ L'équipe de développement
main-login_process =
    À propos du mode multijoueur :

    Veuillez noter que vous avez besoin d'un compte pour jouer sur des serveurs où l'authentification est activée.

    Vous pouvez créer un compte à l'adresse suivante :
    https://veloren.net/account/
main-login-server_not_found = Serveur introuvable.
main-login-authentication_error = Erreur d'authentification sur le serveur.
main-login-internal_error = Erreur interne du client. Remarque : le personnage du joueur a peut-être été supprimé.
main-login-failed_auth_server_url_invalid = Échec de connexion au serveur d'authentification.
main-login-insecure_auth_scheme = Le schéma d'authentification HTTP n'est pas pris en charge. Il n'est pas sûr ! À des fins de développement, HTTP est autorisé pour 'localhost' ou les builds de débogage.
main-login-server_full = Serveur plein.
main-login-untrusted_auth_server = Le serveur d'authentification n'est pas de confiance.
main-login-timeout = Timeout : Le serveur n'a pas répondu à temps. Remarque : il se peut que le serveur soit actuellement surchargé ou qu'il y ait des problèmes sur le réseau.
main-login-server_shut_down = Extinction du Serveur.
main-login-network_error = Problème Réseau.
main-login-network_wrong_version = La version du serveur et du client ne correspond pas. Conseil : vous devez peut-être mettre à jour votre client de jeu.
main-login-failed_sending_request = Demande d'authentification serveur échouée.
main-login-invalid_character = Le personnage sélectionné n'est pas valide.
main-login-client_crashed = Le client a planté.
main-login-not_on_whitelist = Vous ne figurez pas dans la liste blanche du serveur que vous avez tenté de rejoindre.
main-login-banned = Vous avez été banni de façon permanente pour la raison suivante : { $reason }
main-login-kicked = Vous avez été exclu pour le motif suivant : { $reason }
main-login-select_language = Sélectionnez une langue
main-login-client_version = Version du client
main-login-server_version = Version du serveur
main-login-client_init_failed = Le client n'a pas réussi à s'initialiser : { $init_fail_reason }
main-login-username_bad_characters = Le nom d'utilisateur contient des caractères invalides ! (Seuls les caractères alphanumériques, '_' et '-' sont autorisés).
main-login-username_too_long = Nom d'utilisateur trop long ! La taille maximum est : { $max_len }
main-servers-select_server = Sélectionnez un serveur
main-servers-singleplayer_error = Échec de connexion au serveur interne : { $sp_error }
main-servers-network_error = Erreur serveur/socket réseau : { $raw_error }
main-servers-participant_error = Participant déconnecté/erreur de protocole : { $raw_error }
main-servers-stream_error = Erreur de connexion du client/compression/(dé)sérialisation : { $raw_error }
main-servers-database_error = Erreur base de données serveur : { $raw_error }
main-servers-persistence_error = Erreur de persistance du serveur (Probablement liée aux fichiers/données de personnage) : { $raw_error }
main-servers-other_error = Erreur générale du serveur : { $raw_error }
main-credits = Crédits
main-credits-created_by = créé par
main-credits-music = Musiques
main-credits-fonts = Polices d'écriture
main-credits-other_art = Autre Art
main-credits-contributors = Contributeurs
loading-tips =
    .a0 = Appuyez sur '{ $gameinput-togglelantern }' pour allumer votre lanterne.
    .a1 = Appuyez sur '{ $gameinput-controls }' pour voir les raccourcis clavier par défaut.
    .a2 = Vous pouvez écrire /say ou /s pour discuter avec les joueurs directement à côté de vous.
    .a3 = Vous pouvez écrire /region ou /r pour discuter avec les joueurs situés à quelques centaines de blocs autour de vous.
    .a4 = Les administrateurs peuvent utiliser la commande /build pour entrer en mode construction.
    .a5 = Vous pouvez écrire /group ou /g pour discuter uniquement avec les membres de votre groupe actuel.
    .a6 = Pour envoyer un message privé, écrivez /tell suivi par un nom de joueur puis votre message.
    .a7 = Gardez l'oeil ouvert pour trouver de la nourriture, des coffres et autres butins éparpillés dans le monde.
    .a8 = Votre inventaire est rempli de nourriture ? Essayez de créer des plats plus avancés avec !
    .a9 = A court d'idées pour votre prochaine quête ? Essayez de visiter un des donjons marqués sur la carte !
    .a10 = N'oubliez pas d'ajuster les graphismes pour votre système. Appuyez sur '{ $gameinput-settings }' pour ouvrir les paramètres.
    .a11 = Jouer à plusieurs est amusant ! Appuyez sur '{ $gameinput-social }' pour voir qui est en ligne.
    .a12 = Appuyez sur '{ $gameinput-dance }' pour danser. C'est la fête !
    .a13 = Appuyez sur '{ $gameinput-glide }' pour ouvrir votre deltaplane et conquérir les cieux.
    .a14 = Veloren est encore en pré-alpha. Nous faisons de notre mieux pour l'améliorer chaque jour !
    .a15 = Si vous voulez vous joindre à l'équipe de développement ou juste discuter avec nous, rejoignez notre serveur Discord.
    .a16 = Vous pouvez activer l'affichage du nombre de points de vie sur la barre de santé dans les options.
    .a17 = Asseyez-vous près d'un feu de camp (avec la touche '{ $gameinput-sit }') pour vous reposer et régénérer votre santé.
    .a18 = Besoin de plus de sacs ou de meilleures armures pour continuer votre aventure ? Appuyez sur '{ $gameinput-crafting }' pour ouvrir le menu d'artisanat !
    .a19 = Appuyez sur '{ $gameinput-roll }' pour rouler. Faire une roulade peut être utilisé pour se déplacer plus vite et esquiver les attaques ennemies.
    .a20 = Vous vous demandez à quoi sert un objet ? Recherchez 'input:<item name>' dans le menu d'artisanat pour voir les recettes l'utilisant.
    .a21 = Vous pouvez prendre une capture d'écran à l'aide de la touche '{ $gameinput-screenshot }'.
main-singleplayer-new = Nouveau
main-singleplayer-map_shape = Forme
menu-singleplayer-confirm_delete = Êtes-vous sûr de vouloir supprimer "{ $world_name }" ?
main-singleplayer-delete = Supprimer
main-singleplayer-random_seed = Aléatoire
menu-singleplayer-confirm_regenerate = Êtes-vous sûr de vouloir regénérer «{ $world_name }» ?
main-singleplayer-play = Jouer
main-singleplayer-regenerate = Regénérer
main-singleplayer-create_custom = Personnaliser
main-singleplayer-size_lg = Taille logarithmique
main-singleplayer-map_large_warning = Attention : les grands mondes prennent beaucoup de temps pour se lancer la première fois.
main-singleplayer-day_length = Durée de la journée
main-singleplayer-seed = Graine
main-singleplayer-world_name = Nom du monde
main-singleplayer-map_scale = Échelle verticale
main-singleplayer-map_erosion_quality = Qualité de l'érosion
main-singleplayer-generate_and_play = Générer et Jouer
main-server-rules = Ce serveur a des règles qui doivent être acceptées.
main-server-rules-seen-before = Ces règles ont changés depuis la dernière fois que vous les avez accepté.
main-singleplayer-map_shape-circle = Cercle
main-singleplayer-map_shape-square = Carré
main-login-banned_until =
    Vous avez été temporairement banni pour la raison suivante : { $reason }
    Jusqu'au : { $end_date }
main-singleplayer-map_large_extra_warning = Ces paramètres prendraient autant de ressources que générer environ { $count } mondes avec les options par défaut.
