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
