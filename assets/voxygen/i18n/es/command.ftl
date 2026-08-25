command-help-template = { $usage } { $description }
command-adminify-desc = Otorga temporalmente a un jugador el rol de administrador restringido o elimina el actual (si aún no se ha otorgado)
command-spawned-airship = Ha generado un dirigible
command-airship-desc = Genera un dirigible
command-alias-desc = Cambia tu alias
command-help-list =
    { $client-commands }
    { $server-commands }

    Además, puedes utilizar los siguientes atajos
    { $additional-shortcuts }
command-area_add-desc = Añade una nueva área de construcción
command-area_remove-desc = Elimina el área de construcción especificada
command-body-desc = Cambia tu cuerpo a una especie diferente
command-campfire-desc = Crea una hoguera
command-debug_column-desc = Imprime información de depuración sobre una columna
command-disconnect_all_players-desc = Desconecta a todos los jugadores del servidor
command-dropall-desc = Deja caer todos tus objetos al suelo
command-dummy-desc = Genera un muñeco de entrenamiento
command-explosion-desc = Explota el suelo a tu alrededor
command-faction-desc = Envía mensajes a tu facción
command-group-desc = Envía mensajes a tu grupo
command-join_faction-desc = Unirse/abandonar la facción especificada
command-kit-desc = Coloca un conjunto de objetos en tu inventario.
command-area_list-desc = Lista todas las áreas de construcción
command-aura-desc = Crea un aura
command-set_body_type-not_found =
    Ese no es un tipo de cuerpo válido.
    Prueba uno de estos:
    { $options }
command-set_body_type-no_body = No se pudo establecer el tipo de cuerpo ya que el objetivo no tiene un cuerpo.
command-set_body_type-not_character = Solo puede establecer permanentemente un tipo de cuerpo si el objetivo es un jugador conectado como personaje.
command-buff-desc = Aplica un potenciador al jugador
command-build-desc = Activa y desactiva el modo de construcción
command-ban-desc = Bloquea a un jugador con un determinado nombre de usuario, por un periodo determinado (si se proporciona). Indique "true for overwrite" para modificar un bloqueo existente.
command-ban-ip-desc = Bloquea a un determinado jugador, por un periodo de tiempo determinado (si es provisto). A diferencia de un bloqueo normal, este también bloquea la dirección IP asociada con este usuario. Indique "true for overwrite" para modificar un bloqueo existente.
command-battlemode-desc =
    Configura tu modo de batalla a:
    + pvp (jugador vs jugador)
    + pve (jugador vs entorno).
    Si se usa sin argumentos, mostrará el modo de batalla actual.
command-clear_persisted_terrain-desc = Limpia terreno cercano que sea persistente
command-create_location-desc = Crea una ubicación en la posición actual
command-death_effect-dest = Añade un efecto al morir en la entidad objetivo
command-debug_ways-desc = Imprime información de depuración sobre las formas de una columna
command-delete_location-desc = Elimina una ubicación
command-destroy_tethers-desc = Destruye todos los lazos conectados a ti
command-dismount-desc = Desmonta si estás montando, o desmonta cualquier cosa que te monte
command-give_item-desc = Te da algunos objetos. Para ejemplos o auto completar, usa Tab.
command-gizmos-desc = Administra las subscripciones gizmo.
command-gizmos_range-desc = Cambia el rango de las suscripciones gizmo.
command-goto-desc = Teletransporta a una posición
command-goto-rand = Teletransporta a una posición aleatoria
command-group_invite-desc = Invita a un jugador a unirse al grupo
command-group_kick-desc = Remueve a un jugador del grupo
command-group_leave-desc = Abandona el grupo actual
command-group_promote-desc = Promueve un jugador a líder de grupo
command-health-desc = Establece tu salud actual
command-into_npc-desc = Te convierte a ti en un NPC. Ten cuidado!
command-jump-desc = Desplaza tu posición actual
command-kick-desc = Expulsa a un jugador con un nombre de usuario indicado
command-kill-desc = Suicidarte
command-kill_npcs-desc = Mata a los NPCs
command-lantern-desc = Cambia la potencia y color de tu linterna
command-light-desc = Crea una entidad con luz
command-lightning-desc = Caída de un rayo en la posición actual
command-location-desc = Teletransportarse a un lugar
command-make_block-desc = Crea un bloque en tu ubicación con un color
command-make_npc-desc =
    Genera una entidad a partir de la configuración cercana.
    Para ver un ejemplo o autocompletar, pulsa Tab.
command-set_body_type-desc = Selecciona tu tipo de cuerpo, Femenino o Masculino.
command-battlemode_force-desc = Cambia tu estado de combate sin ninguna comprobación
command-experimental-shaders-enabled = Habilitado { $shader }
command-set-build-mode-on-persistent = Se ha activado el modo de construcción. La persistencia experimental del terreno está habilitada. El servidor intentará guardar los cambios, pero no se garantiza que esto funcione.
command-server-no-experimental-terrain-persistence = El servidor se compiló sin la persistencia del terreno habilitada
command-spot-world_feature = Para ejecutar este comando, es necesario habilitar la función `worldgen`.
command-wiki-success = Éxito del comando de Wiki
command-reset_tutorial-desc = Restablecer el tutorial del juego a su estado inicial
command-reset_tutorial-success = Restablecer el estado del tutorial.
command-naga-desc = Cambiar el uso de Naga en el procesamiento inicial del sombreador (no se guarda)
players-list-header =
    { $count ->
        [1]
            { $count } jugador en línea
            { $player_list }
       *[other]
            { $count } jugadores en línea
            { $player_list }
    }
command-clear-desc = Borra todos los mensajes del chat. Afecta a todas las pestañas de chat.
command-experimental_shader-desc = Cambia un sombreador experimental.
command-help-desc = Mostrar información sobre los comandos
command-mute-desc = Silencia los mensajes de chat de un jugador.
command-unmute-desc = Des-silencia une usuarie que se había silenciado con el comando «mute».
command-waypoint-desc = Mostrar la ubicación del punto de ruta actual
command-preprocess-target-error = Se esperaba { $expected_list } después de '@', pero se encontró { $target }
command-preprocess-not-looking-at-valid-target = No se está observando un objetivo válido
command-preprocess-not-selected-valid-target = No se seleccionó un objetivo válido
command-preprocess-not-riding-valid-entity = No se está montando una entidad válida
command-preprocess-not-valid-rider = No hay une jinete válide
command-preprocess-no-player-entity = No hay entidad de jugadore
command-invalid-command-message =
    No se pudo encontrar un comando llamado { $invalid-command }.
    ¿Quizás te referías a alguno de los siguientes?
    { $most-similar-command }
    { $commands-with-same-prefix }

    Escribe /help para ver una lista de todos los comandos.
command-mute-cannot-mute-self = No puedes silenciarte a ti misme
command-mute-success = Se ha silenciado con éxito a { $player }
command-mute-no-player-found = No se pudo encontrar une jugadore llamade { $player }
command-mute-already-muted = { $player } ya está silenciade
command-mute-no-player-specified = Debes especificar une jugadore
command-unmute-cannot-unmute-self = No puedes des-silenciarte a ti misme
command-unmute-success = Se ha des-silenciado a { $player } con éxito
command-unmute-no-muted-player-found = No se pudo encontrar une jugadore silenciade llamade { $player }
command-unmute-no-player-specified = Debes especificar une jugadore a silenciar
command-shader-backend = Backend de sombreador actual: { $shader-backend }
command-experimental-shaders-list = { $shader-list }
command-experimental-shaders-not-found = No hay sombreadores experimentales
command-experimental-shaders-disabled = Deshabilitado { $shader }
command-experimental-shaders-not-supported = { $shader } no es compatible con esta versión del juego
command-experimental-shaders-not-a-shader = { $shader } no es un sombreador experimental; usa este comando con cualquier argumento para ver una lista completa.
command-experimental-shaders-not-valid = Debes especificar un sombreador experimental válido; para obtener una lista de sombreadores experimentales, utiliza este comando sin ningún argumento.
command-no-permission = No tienes permiso para usar '/{ $command_name }'
command-position-unavailable = No se puede obtener la posición de { $target }
command-player-role-unavailable = No se pueden obtener los roles de administrador para { $target }
command-uid-unavailable = No se puede obtener el UID de { $target }
command-area-not-found = No se pudo encontrar el área llamada «{ $area }»
command-player-not-found = ¡No se encontró le jugadore '{ $player }'!
command-player-uuid-not-found = ¡No se encontró le jugadore con el UUID '{ $uuid }'!
command-username-uuid-unavailable = No se pudo determinar el UUID para le nombre de usuarie { $username }
command-uuid-username-unavailable = No se pudo determinar le nombre de usuarie para el UUID  { $uuid }
command-no-sudo = Es de mala educación hacerse pasar por otras personas
command-entity-dead = ¡La entidad '{ $entity }' ha muerto!
command-error-write-settings =
    No se pudo escribir el archivo de configuración en el disco, pero sí en la memoria.
    Error (almacenamiento): { $error }
    Éxito (memoria): { $message }
command-error-while-evaluating-request = Se produjo un error al validar la solicitud: { $error }
command-give-inventory-full =
    El inventario de le jugadore está lleno. Se entregó { $given ->
        [1] solo uno
       *[other] { $given }
    } de { $total } objetos.
command-give-inventory-success = Se agregaron { $total } x { $item } al inventario.
command-invalid-item = Objeto inválido: { $item }
command-invalid-block-kind = Tipo de bloque inválido: { $kind }
command-nof-entities-at-least = El número de entidades debe ser al menos 1
command-nof-entities-less-than = El número de entidades debe ser menor a 50
command-entity-load-failed = No se pudo cargar la configuración de la entidad: { $config }
command-spawned-entities-config = Se generaron { $n } entidades a partir de la configuración: { $config }
command-invalid-sprite = Tipo de sprite inválido: { $kind }
command-time-parse-too-large = { $n } es inválido; no puede tener más de 16 dígitos.
command-time-parse-negative = { $n } es inválido; no puede ser negativo.
command-time-backwards = { $t } es anterior a la hora actual; el tiempo no puede retroceder.
command-time-invalid = { $t } no es una hora válida.
command-time-current = La hora es { $t }
command-time-unknown = Hora desconocida
command-rtsim-purge-perms = Debes ser un administrador real (no solo un administrador temporal) para purgar los datos de rtsim.
command-chunk-not-loaded = No se han cargado los fragmentos { $x }, { $y }
command-chunk-out-of-bounds = El fragmento { $x }, { $y } no se encuentra dentro de los límites del mapa
command-spawned-entity = Entidad creada con el ID: { $id }
command-spawned-dummy = Ha generado un maniquí de entrenamiento
command-spawned-campfire = Ha generado una fogata
command-spawned-safezone = Ha generado una zona segura
command-volume-size-incorrect = El tamaño debe estar entre 1 y 127.
command-volume-created = Ha creado un volumen
command-permit-build-given = Ahora se te permite construir en «{ $area }»
command-permit-build-granted = Se ha otorgado permiso para construir en «{ $area }»
command-revoke-build-recv = Se ha revocado tu permiso para construir en «{ $area }»
command-revoke-build = Se ha revocado el permiso para construir en «{ $area }»
command-revoke-build-all = Se han revocado tus permisos de construcción.
command-revoked-all-build = Se han revocado todos los permisos de construcción.
command-no-buid-perms = No tienes permiso para construir.
command-set-build-mode-off = Se desactivó el modo de construcción.
command-set-build-mode-on-unpersistent = Se ha activado el modo de construcción. Los cambios no se guardarán cuando se descargue un fragmento.
command-set_motd-message-added = El mensaje del día del servidor se ha configurado como { $message }
command-set_motd-message-removed = Se eliminó el mensaje del día del servidor
command-set_motd-message-not-set = No se había establecido ningún mensaje del día
command-set-waypoint-result = ¡Punto de ruta establecido!
command-invalid-alignment = Alineación inválida: { $alignment }
command-kit-not-enough-slots = El inventario no tiene suficientes espacios
command-lantern-unequiped = Por favor, equipa primero una linterna
command-lantern-adjusted-strength = Ajustaste la intensidad de la llama.
command-lantern-adjusted-strength-color = Ajustaste la intensidad y el color de la llama.
command-explosion-power-too-high = La potencia de explosión no debe ser mayor que { $power }
command-explosion-power-too-low = La potencia de explosión debe ser mayor que { $power }
command-disconnectall-confirm =
    Por favor, ejecuta el comando nuevamente con el segundo argumento "confirm" para confirmar que
    realmente deseas desconectar a todos los jugadores del servido
