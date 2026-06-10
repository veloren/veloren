command-help-template = { $usage } { $description }
command-help-list =
    { $client-commands }
    { $server-commands }

    Además, puedes utilizar los siguientes atajos
    { $additional-shortcuts }
command-adminify-desc = Otorga temporalmente a un jugador el rol de administrador restringido o elimina el actual (si aún no se ha otorgado)
command-airship-desc = Genera un dirigible
command-alias-desc = Cambia tu alias
command-area_add-desc = Añade una nueva área de construcción
command-area_list-desc = Lista todas las áreas de construcción
command-area_remove-desc = Elimina el área de construcción especificada
command-aura-desc = Crea un aura
command-body-desc = Cambia tu cuerpo a una especie diferente
command-set_body_type-desc = Selecciona tu tipo de cuerpo, Femenino o Masculino.
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
command-battlemode_force-desc = Cambia tu estado de combate sin ninguna comprobación
command-campfire-desc = Crea una hoguera
command-clear_persisted_terrain-desc = Limpia terreno cercano que sea persistente
command-create_location-desc = Crea una ubicación en la posición actual
command-death_effect-dest = Añade un efecto al morir en la entidad objetivo
command-debug_column-desc = Imprime información de depuración sobre una columna
command-debug_ways-desc = Imprime información de depuración sobre las formas de una columna
command-delete_location-desc = Elimina una ubicación
command-destroy_tethers-desc = Destruye todos los lazos conectados a ti
command-disconnect_all_players-desc = Desconecta a todos los jugadores del servidor
command-dismount-desc = Desmonta si estás montando, o desmonta cualquier cosa que te monte
command-dropall-desc = Deja caer todos tus objetos al suelo
command-dummy-desc = Genera un muñeco de entrenamiento
command-explosion-desc = Explota el suelo a tu alrededor
command-faction-desc = Envía mensajes a tu facción
command-give_item-desc = Te da algunos objetos. Para ejemplos o auto completar, usa Tab.
command-gizmos-desc = Administra las subscripciones gizmo.
command-gizmos_range-desc = Cambia el rango de las suscripciones gizmo.
command-goto-desc = Teletransporta a una posición
command-goto-rand = Teletransporta a una posición aleatoria
command-group-desc = Envía mensajes a tu grupo
command-group_invite-desc = Invita a un jugador a unirse al grupo
command-group_kick-desc = Remueve a un jugador del grupo
command-group_leave-desc = Abandona el grupo actual
command-group_promote-desc = Promueve un jugador a líder de grupo
command-health-desc = Establece tu salud actual
command-into_npc-desc = Te convierte a ti en un NPC. Ten cuidado!
command-join_faction-desc = Unirse/abandonar la facción especificada
command-jump-desc = Desplaza tu posición actual
command-kick-desc = Expulsa a un jugador con un nombre de usuario indicado
command-kill-desc = Suicidarte
command-kill_npcs-desc = Mata a los NPCs
command-kit-desc = Coloca un conjunto de objetos en tu inventario.
command-lantern-desc = Cambia la potencia y color de tu linterna
command-light-desc = Crea una entidad con luz
command-lightning-desc = Caída de un rayo en la posición actual
command-location-desc = Teletransportarse a un lugar
command-make_block-desc = Crea un bloque en tu ubicación con un color
command-make_npc-desc =
    Genera una entidad a partir de la configuración cercana.
    Para ver un ejemplo o autocompletar, pulsa Tab.
command-spawned-airship = Ha generado un dirigible
command-make_sprite-desc = Crea un sprite en tu ubicación; para definir los atributos del sprite, utiliza la sintaxis de Ron para un StructureSprite.
command-make_volume-desc = Crear un volumen (experimental)
command-motd-desc = Ver la descripción del servidor
command-mount-desc = Montar una entidad
command-object-desc = Crear un objeto
command-outcome-desc = Crear un resultado
command-permit_build-desc = Ofrece al jugador un espacio delimitado en el que puede construir
command-players-desc = Lista los jugadores conectados en este momento
command-poise-desc = Establece tu equilibrio actual
command-portal-desc = Crea un portal
command-region-desc = Envía mensajes a todes en tu región del mundo
command-reload_chunks-desc = Vuelve a cargar los fragmentos cargados en el servidor
command-remove_lights-desc = Elimina todas las luces generadas por los jugadores
command-repair_equipment-desc = Repara todos los objetos equipados
command-reset_recipes-desc = Restablece tu libro de recetas
command-respawn-desc = Teletranspórtate a tu punto de ruta
command-revoke_build-desc = Revoca el permiso de construcción de área del jugador
command-revoke_build_all-desc = Revoca todos los permisos de área de construcción del jugador
command-safezone-desc = Crea una zona segura
command-say-desc = Envía mensajes a todos los que estén a un grito de distancia
command-scale-desc = Ajusta el tamaño de tu personaje
command-server_physics-desc = Activar/desactivar las físicas de autoridad del servidor para una cuenta
command-set_motd-desc = Establece la descripción del servidor
command-set-waypoint-desc = Establece tu punto de referencia en tu ubicación actual.
command-ship-desc = Genera una nave
command-site-desc = Teletransportarse a un sitio
command-skill_point-desc = Te das puntos de habilidad para un árbol de habilidades concreto
command-skill_preset-desc = Otorga a tu personaje las habilidades deseadas.
command-spawn-desc = Crear una entidad de prueba
command-spot-desc = Busca y teletranspórtate al lugar más cercano de un tipo determinado.
command-sudo-desc = Ejecuta el comando como si fueras otra entidad
command-tell-desc = Enviar un mensaje a otro jugador
command-tether-desc = Vincula a otra entidad a ti mismo
command-time-desc = Establece la hora del día
command-time_scale-desc = Establecer la escala del tiempo delta
command-tp-desc = Teletransportarse a otra entidad
command-rtsim_chunk-desc = Mostrar información sobre el fragmento actual de rtsim
command-rtsim_info-desc = Mostrar información sobre un rtsim de NPC
command-rtsim_npc-desc = Enumera los rtsim de NPC que se ajusten a una consulta determinada (ejemplo: simulado, comerciante) ordenados por distancia
command-rtsim_purge-desc = Borrar los datos de rtsim en el próximo inicio
command-rtsim_tp-desc = Teletransportarse a un rtsim de npc
command-unban-desc = Elimina el bloqueo del nombre de usuario indicado. Si hay un bloqueo de IP asociado, también se eliminará.
command-unban-ip-desc = Elimina únicamente el bloqueo de IP asociado a ese nombre de usuario.
command-version-desc = Indica la versión del servidor
command-weather_zone-desc = Crear una zona climática
command-whitelist-desc = Añade o elimina un nombre de usuario a la lista blanca
command-wiring-desc = Crear elemento de cableado
command-world-desc = Envia mensajes a todos los usuarios del servidor
command-wiki-desc = Abre la wiki o busca un tema
command-reset_tutorial-desc = Restablecer el tutorial del juego a su estado inicial
command-reset_tutorial-success = Restablecer el estado del tutorial.
command-naga-desc = Activar o desactivar el uso de Naga en el procesamiento inicial del sombreador (no se guarda)
players-list-header =
    { $count ->
        [1]
            { $count } jugador en línea
            { $player_list }
       *[other]
            { $count } jugadores en líne
            { $player_list }
    }
command-clear-desc = Borra todos los mensajes del chat. Afecta a todas las pestañas del chat.
command-experimental_shader-desc = Activa o desactiva un sombreador experimental.
command-help-desc = Mostrar información sobre los comandos
command-mute-desc = Silencia los mensajes de chat de un jugador.
command-unmute-desc = Desactiva el silencio de un jugador que se había silenciado con el comando «mute».
command-waypoint-desc = Mostrar la ubicación del punto de ruta actual
command-preprocess-target-error = Se espera { $expected_list } después de '@' encontrado { $target }
command-preprocess-not-looking-at-valid-target = No se está apuntando a un objetivo válido
command-preprocess-not-selected-valid-target = No se ha seleccionado un objetivo válido
command-preprocess-not-valid-viewpoint-entity = No se está visualizando desde una entidad de punto de vista válida
command-preprocess-not-riding-valid-entity = No se está montando una entidad válida
command-preprocess-not-valid-rider = No hay ningún jinete válido
command-preprocess-no-player-entity = No hay entidad de jugador
command-invalid-command-message =
    No se encontró el comando { $invalid-command }.
    ¿Quizás te refieres a alguno de los siguientes?
    { $most-similar-command }
    { $commands-with-same-prefix }

    Escribe /help para ver una lista de todos los comandos.
command-mute-cannot-mute-self = No puedes silenciarte
command-mute-success = Se ha silenciado correctamente a { $player }
command-mute-no-player-found = No se ha encontrado ningún jugador llamado { $player }
command-mute-already-muted = { $player } ya está silenciado
command-mute-no-player-specified = Debes especificar un jugador
command-unmute-cannot-unmute-self = No puedes quitarte un silenciado a ti
command-unmute-success = Se han reactivado mensajes de { $player }
