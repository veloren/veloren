main-username = Usuario
main-server = Servidor
main-password = Contraseña
main-connecting = Conectando
main-creating_world = Creando mundo
main-tip = Consejo:
main-unbound_key_tip = sin definir
main-notice =
    ¡Bienvenido a la versión alfa de Veloren!

    Antes de que te adentres en el juego, por favor ten en cuenta un par de cosas:

    - Esto es una alfa muy temprana. Espera errores, mecánicas de juego inacabadas, mecánicas sin pulir, y cosas que simplemente faltan.

    - Si tienes críticas constructivas o reportes de errores, puedes contactar con nosotros en GitLab, o en nuestro servidor de Discord o de Matrix.

    - Veloren es un juego de código abierto. Esto quiere decir que eres libre de jugar, modificar y redistribuir el juego en acuerdo con los términos y condiciones de la versión 3 de la licencia "GNU General Public License".

    - Veloren es un proyecto comunitario sin ánimo de lucro, y todo aquel que trabaja en él es un voluntario.
    Si te gusta lo que ves, ¡eres bienvenido a unirte a nuestros grupos de trabajo!

    Gracias por dedicar tu tiempo para leer esta noticia, ¡esperamos que disfrutes del juego!

    ~ El equipo de desarrollo
main-login_process =
    Información sobre el modo multijugador:

    Por favor, ten en cuenta que necesitas una cuenta para jugar en servidores con autenticación activada.

    Puedes crear una cuenta en:
    https://veloren.net/account/
main-login-server_not_found = Servidor no encontrado.
main-login-authentication_error = Error de autenticación al servidor.
main-login-internal_error = Error interno en el cliente. Consejo: puede ser que el personaje esté borrado.
main-login-failed_auth_server_url_invalid = Fallo al conectar con el servidor de autenticación.
main-login-insecure_auth_scheme = El esquema de autenticación mediante HTTP no está soportado. ¡Es inseguro! Solo se permite para asuntos de desarrollo, para 'localhost' o en versiones de depuración.
main-login-server_full = El servidor está lleno.
main-login-untrusted_auth_server = El servidor de autenticación no es de confianza.
main-login-outdated_client_or_server = ServerEnloquecido: Probablemente las versiones son incompatibles, revisa si hay actualizaciones.
main-login-timeout = Timeout: El servidor no respondió a tiempo. Consejo: el servidor podría estar sobrecargado en este momento o el problema podría ser con la red.
main-login-server_shut_down = Servidor apagado.
main-login-network_error = Error de red.
main-login-network_wrong_version = El servidor está ejecutando una versión del juego diferente a la tuya. Consejo: puede que necesites actualizar la versión de tu cliente del juego.
main-login-failed_sending_request = Petición al servidor de autenticación fallida.
main-login-invalid_character = El personaje seleccionado no es válido.
main-login-client_crashed = El cliente se cerró inesperadamente.
main-login-not_on_whitelist = No estás en la lista blanca del servidor al que has intentado unirte.
main-login-banned = Has sido baneado por la siguiente razón:
main-login-kicked = Has sido expulsado por la siguiente razón:
main-login-select_language = Selecciona un idioma
main-login-client_version = Versión del cliente
main-login-server_version = Versión del servidor
main-login-client_init_failed = Fallo del cliente al inicializar: { $init_fail_reason }
main-login-username_bad_characters = ¡El nombre de usuario contiene caracteres inválidos! (Solo alfanuméricos, '_' y '-' están permitidos).
main-login-username_too_long = ¡El nombre de usuario es demasiado largo! La máxima longitud es: { $max_len }
main-servers-select_server = Selecciona un servidor
main-servers-singleplayer_error = Fallo al conectar con el servidor interno: { $sp_error }
main-servers-network_error = Red de servidor/Error de socket: { $raw_error }
main-servers-participant_error = Desconexión de participante/error protocolo: { $raw_error }
main-servers-stream_error = Conexión de cliente/compression/error (de)serialización: { $raw_error }
main-servers-database_error = Error en la base de datos del servidor: { $raw_error }
main-servers-persistence_error = Error de persistencia del servidor (Probablemente datos de Asset/Personaje): { $raw_error }
main-servers-other_error = Error general del servidor : { $raw_error }
main-credits = Créditos
main-credits-created_by = creado por
main-credits-music = Música
main-credits-fonts = Fuentes
main-credits-other_art = Otros artistas
main-credits-contributors = Colaboradores
loading-tips =
    .a0 = Pulsa '{ $gameinput-togglelantern }' para encender tu linterna.
    .a1 = Pulsa '{ $gameinput-help }' para ver todos los atajos de teclado.
    .a2 = Puedes emplear el comando /say o /s para chatear con jugadores que se encuentren justo a tu lado.
    .a3 = Puedes emplear el comando /region o /r para chatear con jugadores que se encuentren a menos de 200 bloques de tu alrededor.
    .a4 = Los administradores pueden usar el comando /build para entrar en el modo de construcción.
    .a5 = Puedes escribir /group o /g para chatear con jugadores de tu grupo.
    .a6 = Para enviar mensajes privados escribe /tell seguido del nombre del jugador y tu mensaje.
    .a7 = ¡Estate atento a la comida, cofres y otros botines esparcidos por el mundo!
    .a8 = ¿Inventario lleno de comida? ¡Intenta procesarla para conseguir mejores alimentos!
    .a9 = ¿Aburrido? ¡Intenta completar una de las mazmorras marcadas en el mapa!
    .a10 = No te olvides de ajustar los gráficos. Pulsa '{ $gameinput-settings }' para abrir la configuración.
    .a11 = ¡Jugar con otros jugadores es divertido! Pulsa '{ $gameinput-social }' para ver quien esta en línea.
    .a12 = Pulsa '{ $gameinput-dance }' para bailar. ¡Fiesta!
    .a13 = Pulsa '{ $gameinput-glide }' para utilizar tu paravela y conquistar los cielos.
    .a14 = Veloren se encuentra todavia en pre-alfa. ¡Hacemos todo lo posible para mejorar la experiencia de juego día a día!
    .a15 = Si quieres unirte al equipo de desarrollo o conversar con nosotros, únete a nuestro servidor de Discord.
    .a16 = Puedes mostrar u ocultar tu total de salud de la barra de salud en los ajustes.
    .a17 = Siéntate cerca de una hoguera (con la tecla '{ $gameinput-sit }') para recuperarte de tus heridas.
    .a18 = ¿Necesitas más bolsas de almacenamiento o mejores armaduras para continuar tu viaje? Pulsa '{ $gameinput-crafting }' para abrir el menú de fabricación.
    .a19 = Pulsa '{ $gameinput-roll }' para rodar por el suelo. Rodar te sirve para esquivar los ataques enemigos y para moverte más rápido.
    .a20 = ¿Para qué sirve este objeto? Busca 'input:<item name>' en fabricación para ver en qué recetas se usa.
    .a21 = ¡Eh, mira eso! Toma un pantallazo pulsando '{ $gameinput-screenshot }'.
main-singleplayer-regenerate = Recrear
main-singleplayer-create_custom = Personalizado
main-singleplayer-invalid_name = Error: nombre no válido
main-singleplayer-seed = Semilla
main-singleplayer-random_seed = Aleatoria
main-singleplayer-new = Nuevo
main-server-rules-seen-before = Las normas del servidor han cambiado desde la última vez que las aceptaste.
main-singleplayer-delete = Borrar
main-server-rules = Este servidor requiere aceptar sus normas.
main-singleplayer-play = Jugar
main-singleplayer-day_length = Duración del día
main-singleplayer-size_lg = Escala logarítmica
main-singleplayer-map_large_warning = Aviso: Los mundos de gran tamaño tardan más tiempo en arrancar por primera vez.
main-singleplayer-world_name = Nombre del mundo
main-singleplayer-map_scale = Escala vertical
main-singleplayer-map_erosion_quality = Calidad de erosión
main-singleplayer-map_shape = Forma
main-singleplayer-generate_and_play = Crear y jugar
menu-singleplayer-confirm_regenerate = ¿Seguro que quieres recrear el mundo "{ $world_name }"?
menu-singleplayer-confirm_delete = ¿Seguro que quieres borrar el mundo "{ $world_name }"?
