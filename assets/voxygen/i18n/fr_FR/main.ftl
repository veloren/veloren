main-username = Nom d'utilisateur
main-server = Serveur
main-password = Mot de passe
main-connecting = Connexion
main-creating_world = Création du monde
main-tip = Astuce:
main-unbound_key_tip = non lié
main-notice =
    Bienvenue dans la version alpha de Veloren !
    
    Avant de commencer à vous amuser, merci de garder les choses suivantes en tête :
    
    - Il s'agit d'une version alpha très jeune. Attendez-vous à des bugs, un gameplay non terminé, des mécaniques non peaufinées et des fonctionalités manquantes.
    
    - Si vous avez des retours constructifs ou avez detecté un bug, vous pouvez nous contacter via Reddit, GitLab ou notre serveur communautaire Discord.
    
    - Veloren est un logiciel open-source sous licence GPL3. Cela signifit que vous êtes libre de jouer, modfier et redistribuer le jeu comme il vous semble (licence contaminante sous GPL 3 pour toute modification)
    
    - Veloren est un projet communautaire à but non-lucratif développé par des bénévoles.
    Si vous appréciez ce jeu, vous êtes les bienvenus pour rejoindre les équipes de développement ou d'artistes!
    
    Merci d'avoir pris le temps de lire cette annonce, nous espérons que vous apprécierez le jeu!
    
    ~ L'équipe de Veloren
main-login_process =
    Information sur la procédure de connexion:
    
    Vous devez à présent posséder un compte
    afin de jouer sur les serveurs avec authentification.
    
    Vous pouvez créer un compte à l'adresse 
    
    https://veloren.net/account/.
main-login-server_not_found = Serveur introuvable
main-login-authentication_error = Erreur d'authentification sur le serveur
main-login-internal_error = Erreur interne du client (Certainement suite à la suppression d'un personnage)
main-login-failed_auth_server_url_invalid = Échec de connexion au serveur d'authentification
main-login-insecure_auth_scheme = Le schéma d'authentification HTTP n'est PAS pris en charge. Ce n'est pas sécurisé ! À des fins de développement, HTTP est autorisé pour 'localhost' ou les build de débogage.
main-login-server_full = Serveur plein
main-login-untrusted_auth_server = Le serveur d'authentification n'est pas de confiance
main-login-outdated_client_or_server = ServeurPasContent: Les versions sont probablement incompatibles, verifiez les mises à jour.
main-login-timeout = DélaiEcoulé: Le serveur n'a pas répondu à temps. (Surchage ou problèmes réseau).
main-login-server_shut_down = Extinction du Serveur
main-login-network_error = Problème Réseau
main-login-network_wrong_version = Le serveur fonctionne avec une version différente de la vôtre. Vérifiez votre version et mettez votre jeu à jour.
main-login-failed_sending_request = Demande d'authentification serveur échouée
main-login-invalid_character = Le personnage sélectionné n'est pas valide
main-login-client_crashed = Le client a planté
main-login-not_on_whitelist = Vous devez être ajouté à la liste blanche par un Admin pour pouvoir entrer
main-login-banned = Vous avez été banni pour la raison suivante
main-login-kicked = Vous avez été exclus pour la raison suivante
main-login-select_language = Sélectionnez une langue
main-login-client_version = Version du client
main-login-server_version = Version du serveur
main-login-client_init_failed = Le client n'a pas réussi à s'initialiser: { $init_fail_reason }
main-login-username_bad_characters = Le pseudo contient des caractères invalides! (Seulement alphanumériques, '_' et '-' sont autorisés)
main-login-username_too_long = Pseudo trop long! La taille maximum est: { $max_len }
main-servers-select_server = Sélectionnez un serveur
main-servers-singleplayer_error = Échec de connexion au serveur interne: { $sp_error }
main-servers-network_error = Réseau serveur/socket erreur: { $raw_error }
main-servers-participant_error = Participant déconnecté/erreur protocole: { $raw_error }
main-servers-stream_error = Connexion du client/compression/(dé)sérialisation erreur: { $raw_error }
main-servers-database_error = Erreur base de données serveur: { $raw_error }
main-servers-persistence_error = Erreur serveur persistante (Probablement données Asset/Character liées): { $raw_error }
main-servers-other_error = Erreur général serveur: { $raw_error }
main-credits = Crédits
main-credits-created_by = créé par
main-credits-music = Musiques
main-credits-fonts = Polices d'écriture
main-credits-other_art = Autre Art
main-credits-contributors = Contributeurs
loading-tips =
    .a0 = Appuyez sur '{ $gameinput-togglelantern }' pour allumer ta lanterne.
    .a1 = Appuyez sur '{ $gameinput-help }' pour voir les raccourcis clavier par défaut.
    .a2 = Vous pouvez taper /say ou /s pour discuter aux joueurs directement à côté toi.
    .a3 = Vous pouvez taper /region ou /r pour discuter avec les joueurs situés à quelques centaines de blocs de toi.
    .a4 = Pour envoyer un message privé, tapez /tell suivi par un nom de joueur puis votre message.
    .a5 = Des PNJs avec le même niveau peuvent varier en difficulté.
    .a6 = Regardez le sol pour trouver de la nourriture, des coffres et d'autres butins !
    .a7 = Votre inventaire est rempli de nourriture ? Essayez de créer un meilleur repas avec !
    .a8 = Vous cherchez une activité ? Essayez de visiter un des donjons marqués sur la carte !
    .a9 = N'oubliez pas d'ajuster les graphismes pour votre système. Appuyez sur '{ $gameinput-settings }' pour ouvrir les paramètres.
    .a10 = Jouer à plusieurs est amusant ! Appuyez sur '{ $gameinput-social }' pour voir qui est en ligne.
    .a11 = Un PNJ avec une tête de mort sous sa barre de vie est plus puissant que vous.
    .a12 = Appuyez sur '{ $gameinput-dance }' pour danser. C'est la fête !
    .a13 = Appuyez sur '{ $gameinput-glide }' pour ouvrir votre deltaplane et conquérir les cieux.
    .a14 = Veloren est encore en Pré-Alpha. Nous faisons de notre mieux pour l'améliorer chaque jour !
    .a15 = Si vous voulez vous joindre à l'équipe de développement ou juste discuter avec nous, rejoignez notre serveur Discord.
    .a16 = Vous pouvez afficher ou non combien de santé vous avez dans les options.
    .a17 = Pour voir vos statistiques, cliquez sur le bouton 'Stats' dans l'inventaire
    .a18 = Asseyez-vous près d'un feu de camp (avec la touche '{ $gameinput-sit }') pour vous reposer - cela régénèrera votre santé.
    .a19 = Besoin de plus de sacs ou de meilleures armures pour continuer votre aventure ? Appuyez sur '{ $gameinput-crafting }' pour ouvrir le menu de craft!
    .a20 = Appuyez sur '{ $gameinput-roll }' pour rouler. Faire une roulade peut être utilisé pour se déplacer plus vite et esquiver les attaques ennemies.
    .a21 = Vous vous demandez à quoi sert un objet ? Rechercher 'input:<item name>' dans le menu de craft pour voir dans quelle(s) recette(s) il est utilisé.
    .a22 = Vous avez trouver quelque chose de cool ? Prenez une capture d'écran à l'aide de '{ $gameinput-screenshot }'.