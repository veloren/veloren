hud-show_tips = Mostrar consejos
hud-quests = Misiones
hud-you_died = Has muerto
hud-waypoint_saved = Punto de control guardado
hud-sp_arrow_txt = PH
hud-inventory_full = Inventario lleno
hud-someone_else = alguien más
hud-another_group = otro grupo
hud-owned_by_for_secs = Este botín le pertenece a { $name } durante { $secs } segundos
hud-press_key_to_show_debug_info_fmt = Pulsa { $key } para mostrar la información de depuración
hud-press_key_to_toggle_keybindings_fmt = Pulsa { $key } para alternar atajos de teclado
hud-press_key_to_toggle_debug_info_fmt = Pulsa { $key } para alternar la información de depuración
hud-press_key_to_respawn = Pulsa { $key } para reaparecer en la última hoguera que hayas visitado.
hud-tutorial_btn = Tutorial
hud-tutorial_click_here = ¡Pulsa [ { $key } ] para liberar tu cursor y hacer click sobre este botón!
hud-tutorial_elements = Fabricación
hud-temp_quest_headline = ¡Saludos, viajero!
hud-temp_quest_text =
    Para comenzar tu viaje puedes empezar por explorar esta aldea y recoger suministros.

    ¡Puedes llevarte todo lo que necesites para tu viaje!

    En la parte inferior derecha de la pantalla encontrarás cosas como tu bolsa, el menú de fabricación y el mapa.

    El menú de fabricación te permite crear armaduras, armas, comida, ¡y mucho más!

    Los animales salvajes que rodean la aldea son una buena fuente de pieles, que te permiten crear armaduras para defenderte de los peligros del mundo.

    Cuando te sientas preparado, ¡intenta conseguir equipamiento aún mejor dentro de las mazmorras y las cuevas marcadas en tu mapa!
hud-spell = Conjuros
hud-diary = Diario
hud-free_look_indicator =
    { $toggle ->
        [0] Vista libre activa. Pulsa { $key } para desactivarla.
       *[other] Vista libre activa. Deja de presionar { $key } para desactivarla.
    }
hud-camera_clamp_indicator = Cámara fija vertical activa. Pulsa { $key } para desactivarla.
hud-auto_walk_indicator = Avance automático activo
hud-zoom_lock_indicator-remind = Zoom fijado
hud-zoom_lock_indicator-enable = Zoom de cámara fijado
hud-zoom_lock_indicator-disable = Zoom de cámara desbloqueado
hud-collect = Recolectar
hud-pick_up = Recoger
hud-open = Abrir
hud-use = Usar
hud-mine = Picar
hud-mine-needs_pickaxe = Se necesita un pico
hud-talk = Hablar
hud-trade = Comerciar
hud-mount = Montar
hud-sit = Sentarse
hud-mine-needs_shovel = Requiere una pala
hud-unlock-requires = Abrir con { $item }
hud-read = Leer
hud-stay = Quedarse
hud-unlock-consumes = Utiliza { $item } para abrir
hud-portal = Portal
hud-follow = Seguir
hud-items_lost_dur = Tus objetos equipados han perdido durabilidad.
hud-items_will_lose_dur = Tus objetos equipados perderán durabilidad.
hud-hardcore_char_deleted = Este personaje extremo ha sido eliminado.
hud-hardcore_will_char_deleted = Este personaje extremo será eliminado.
hud-press_key_to_give_up = Mantén pulsado { $key } para rendirte y morir.
hud-press_key_to_return_to_char_menu = Presiona { $key } para volver al menú de personajes.
hud-downed_recieving_help = Recibiendo ayuda.
hud-activate = Activar
hud-deactivate = Desactivar
hud-steal = Robar
hud-steal-requires = Roba con { $item }
hud-steal-consumes = Usa { $item } para robar
hud-dig = Cavar
hud-mine-needs_unhandled_case = Se necesita ???
hud-help = Ayuda
hud-pet = Acariciar
hud-waypoint_interact = Establecer Punto de Referencia
hud-steer = Conducir
hud-rest = Descansa
-server = Servidor
-client = Cliente
hud-init-stage-singleplayer = Iniciando servidor en modo de un jugador...
hud-init-stage-server-db-migrations = { "[" }{ -server }]: Realizando migraciones en la base de datos...
hud-init-stage-server-db-vacuum = { "[" }{ -server }]: Limpiando la base de datos...
hud-init-stage-server-worldsim-erosion = { "[" }{ -server }]: Erosión al { $percentage } %
hud-init-stage-server-worldsim-erosion_time_left =
    .days =
        { $n ->
            [one] ~{ $n } día restante
           *[other] ~{ $n } días restantes
        }
    .hours =
        { $n ->
            [one] ~{ $n } hora restante
           *[other] ~{ $n } horas restantes
        }
    .minutes =
        { $n ->
            [one] ~{ $n } minuto restante
           *[other] ~{ $n } minutos restantes
        }
    .seconds =
        { $n ->
            [one] ~{ $n } segundo restante
           *[other] ~{ $n } segundos restantes
        }
hud-init-stage-server-worldciv-civcreate = { "[" }{ -server }]: Se han creado { $generated } de { $total } civilizaciones
hud-init-stage-server-worldciv-site = { "[" }{ -server }]: Generando ubicaciones...
hud-init-stage-server-economysim = { "[" }{ -server }]: Simulando la economía...
hud-init-stage-server-spotgen = { "[" }{ -server }]: Generando sitios...
hud-init-stage-server-starting = { "[" }{ -server }]: Iniciando el servidor...
hud-init-stage-multiplayer = Iniciando el modo multijugador
hud-init-stage-client-connection-establish = { "[" }{ -client }]: Estableciendo conexión con el servidor...
hud-init-stage-client-request-server-version = { "[" }{ -client }]: Esperando a recibir la versión del servidor...
hud-init-stage-client-authentication = { "[" }{ -client }]: Iniciando sesión...
hud-init-stage-client-load-init-data = { "[" }{ -client }]: Cargando datos de inicialización desde el servidor...
hud-init-stage-client-starting-client = { "[" }{ -client }]: Preparando al cliente...
hud-init-stage-render-pipeline = Creando tubería de renderizado ({ $done }/{ $total })
hud-tutorial-disable = Desactivar permanentemente las sugerencias del tutorial
