# General - Todos los árboles de habilidades
hud-rank_up = Nuevo punto de habilidad adquirido
hud-skill-sp_available =
    { $number ->
        [0] Sin puntos de habilidad disponibles
        [one] { $number } punto de habilidad disponible
        *[other] { $number } puntos de habilidad disponibles
    }
hud-skill-not_unlocked = Bloqueado
hud-skill-req_sp ={"\u000A"}
    
    Requiere { $number ->
        [one] { $number } punto de habilidad
        *[other] { $number } puntos de habilidad
    }

hud-skill-set_as_exp_bar = Fijar en barra de experiencia

# Combate general - Árbol de habilidades
## Salud 
hud-skill-inc_health_title = Aumentar salud
hud-skill-inc_health = Aumenta la salud máxima en { $boost } puntos{ $SP }

## Aguante
hud-skill-inc_energy_title = Aumentar aguante
hud-skill-inc_energy = Aumenta el aguante máximo en { $boost } puntos{ $SP }

## Competencia con armas
hud-skill-unlck_sword_title = Competencia con espadas
hud-skill-unlck_sword = Desbloquea el árbol de habilidades de la espada{ $SP }
hud-skill-unlck_axe_title = Competencia con hachas
hud-skill-unlck_axe = Desbloquea el árbol de habilidades del hacha{ $SP }
hud-skill-unlck_hammer_title = Competencia con martillos
hud-skill-unlck_hammer = Desbloquea el árbol de habilidades del martillo{ $SP }
hud-skill-unlck_bow_title = Competencia con arcos
hud-skill-unlck_bow = Desbloquea el árbol de habilidades del arco{ $SP }
hud-skill-unlck_staff_title = Competencia con bastones
hud-skill-unlck_staff = Desbloquea el árbol de habilidades del bastón{ $SP }
hud-skill-unlck_sceptre_title = Competencia con cetros
hud-skill-unlck_sceptre = Desbloquea el árbol de habilidades del cetro{ $SP }

## Esquiva
hud-skill-dodge_title = Esquivar
hud-skill-dodge = Ruedas por el suelo para darte un breve período de invulnerabilidad y así poder esquivar los ataques enemigos.
hud-skill-roll_energy_title = Coste de aguante para esquivar
hud-skill-roll_energy = Esquivar consume un { $boost } % menos de aguante{ $SP }
hud-skill-roll_speed_title = Velocidad al esquivar
hud-skill-roll_speed = Te desplazas un { $boost } % más rápido al rodar por el suelo{ $SP }
hud-skill-roll_dur_title = Duración de esquiva
hud-skill-roll_dur = Tu esquiva dura un { $boost } % más{ $SP }

## Escalada
hud-skill-climbing_title = Escalar
hud-skill-climbing = Subir pendientes y trepar a grandes alturas
hud-skill-climbing_cost_title = Coste de aguante para escalar
hud-skill-climbing_cost = Escalar consume un { $boost } % menos de aguante{ $SP }
hud-skill-climbing_speed_title = Velocidad de escalada
hud-skill-climbing_speed = Escalas un { $boost } % más rápido{ $SP }

## Nado
hud-skill-swim_title = Nadar
hud-skill-swim = Movimiento acuático
hud-skill-swim_speed_title = Velocidad al nadar
hud-skill-swim_speed = Nadas un { $boost } % más rápido{ $SP }

# Martillo - Árbol de habilidades
## Golpe único
hud-skill-hmr_single_strike_title = Golpe sencillo
hud-skill-hmr_single_strike = Tan sencillo como tú
hud-skill-hmr_single_strike_knockback_title = Retroceso de {{ hud-skill-hmr_single_strike_title }}
hud-skill-hmr_single_strike_knockback = Aumenta el retroceso de los golpes en un { $boost } %{ $SP }
hud-skill-hmr_single_strike_regen_title = Regeneración de {{ hud-skill-hmr_single_strike_title }}
hud-skill-hmr_single_strike_regen = Aumenta el aguante ganado con cada golpe sucesivo{ $SP }
hud-skill-hmr_single_strike_damage_title = Daño de {{ hud-skill-hmr_single_strike_title }}
hud-skill-hmr_single_strike_damage = Aumenta el daño infligido con cada golpe sucesivo{ $SP }
hud-skill-hmr_single_strike_speed_title = Velocidad de {{ hud-skill-hmr_single_strike_title }}
hud-skill-hmr_single_strike_speed = Aumenta la velocidad de ataque con cada golpe sucesivo{ $SP }

## Martillazo
hud-skill-hmr_charged_melee_title = Martillazo
hud-skill-hmr_charged_melee = Un golpe más con el martillo... pero esta vez lleno de energía
hud-skill-hmr_charged_rate_title = Velocidad de carga de {{ hud-skill-hmr_charged_melee_title }}
hud-skill-hmr_charged_rate = El tiempo para preparar un martillazo es un { $boost } % más rápido{ $SP }
hud-skill-hmr_charged_melee_nrg_drain_title = Consumo de aguante de {{ hud-skill-hmr_charged_melee_title }}
hud-skill-hmr_charged_melee_nrg_drain = Reduce la velocidad con la que se consume el aguante mientras se prepara un golpe cargado con el martillo en un { $boost } %{ $SP }
hud-skill-hmr_charged_melee_damage_title = Daño de {{ hud-skill-hmr_charged_melee_title }}
hud-skill-hmr_charged_melee_damage = Aumenta el daño del golpe cargado en un { $boost } %{ $SP }
hud-skill-hmr_charged_melee_knockback_title = Retroceso de {{ hud-skill-hmr_charged_melee_title }}
hud-skill-hmr_charged_melee_knockback = Aumenta en gran medida el potencial para lanzar por los aires a los enemigos en un { $boost } %{ $SP }

## Terremoto
-hud-skill-hmr_leap_title = Terremoto
hud-skill-hmr_unlock_leap_title = Desbloquear {{ -hud-skill-hmr_leap_title }}
hud-skill-hmr_unlock_leap = Desbloquea el {{ -hud-skill-hmr_leap_title }}{ $SP }
hud-skill-hmr_leap_damage_title = Daño de {{ -hud-skill-hmr_leap_title }}
hud-skill-hmr_leap_damage = Aumenta el daño del salto en un { $boost } %{ $SP }
hud-skill-hmr_leap_distance_title = Distancia de {{ -hud-skill-hmr_leap_title }}
hud-skill-hmr_leap_distance = Aumenta la distancia de salto en un { $boost } %{ $SP }
hud-skill-hmr_leap_knockback_title = Retroceso de {{ -hud-skill-hmr_leap_title }}
hud-skill-hmr_leap_knockback = Aumenta el retroceso infligido del salto en un { $boost } %{ $SP }
hud-skill-hmr_leap_cost_title = Coste de {{ -hud-skill-hmr_leap_title }}
hud-skill-hmr_leap_cost = Reduce el coste del salto en un { $boost } %{ $SP }
hud-skill-hmr_leap_radius_title = Radio de {{ -hud-skill-hmr_leap_title }}
hud-skill-hmr_leap_radius = Aumenta el radio del golpe al suelo en { $boost } metros{ $SP }

# Hacha - Árbol de habilidades
## Golpe doble
hud-skill-axe_double_strike_title = Golpe doble
hud-skill-axe_double_strike = Haz picadillo a esos villanos
hud-skill-axe_double_strike_combo_title = Golpe triple
hud-skill-axe_double_strike_combo = Desbloquea un golpe adicional{ $SP }
hud-skill-axe_double_strike_regen_title = Regeneración de {{ hud-skill-axe_double_strike_title }} 
hud-skill-axe_double_strike_regen = Aumenta la ganancia de aguante con cada golpe sucesivo{ $SP }
hud-skill-axe_double_strike_damage_title = Daño de {{ hud-skill-axe_double_strike_title }}
hud-skill-axe_double_strike_damage = Aumenta el daño infligido con cada golpe sucesivo{ $SP }
hud-skill-axe_double_strike_speed_title = Velocidad de {{ hud-skill-axe_double_strike_title }}
hud-skill-axe_double_strike_speed = Aumenta la velocidad de ataque con cada golpe sucesivo{ $SP }

## Giro de hacha
hud-skill-axe_spin_title = Giro de hacha
hud-skill-axe_spin = Haces girar el hacha...
hud-skill-axe_infinite_axe_spin_title = {{ hud-skill-axe_spin_title }} infinito
hud-skill-axe_infinite_axe_spin = Gira durante tanto tiempo como aguante tengas{ $SP }
hud-skill-axe_spin_speed_title = Velocidad de {{ hud-skill-axe_spin_title }}
hud-skill-axe_spin_speed = Aumenta tu velocidad de giro en un { $boost } %{ $SP }
hud-skill-axe_spin_damage_title = Daño de {{ hud-skill-axe_spin_title }}
hud-skill-axe_spin_damage = Aumenta el daño que hace cada giro en un { $boost } %{ $SP }
hud-skill-axe_spin_helicopter_title = Helicóptero
hud-skill-axe_spin_helicopter = Caes un poco más lento mientras giras{ $SP }
hud-skill-axe_spin_cost_title = Coste de {{ hud-skill-axe_spin_helicopter_title }}
hud-skill-axe_spin_cost = Reduce el coste de aguante de los giros en un { $boost } %{ $SP }

## Salto con hacha
-hud-skill-axe_unlock_title = Salto con hacha
hud-skill-axe_unlock_leap_title = Desbloquear {{ -hud-skill-axe_unlock_title }}
hud-skill-axe_unlock_leap = Desbloquea el salto giratorio{ $SP }
hud-skill-axe_leap_damage_title = Daño de {{ -hud-skill-axe_unlock_title }}
hud-skill-axe_leap_damage = Aumenta el daño del salto en un { $boost } %{ $SP }
hud-skill-axe_leap_distance_title = Distancia de {{ -hud-skill-axe_unlock_title }}
hud-skill-axe_leap_distance = Aumenta la distancia del salto en un { $boost } %{ $SP }
hud-skill-axe_leap_knockback_title = Retroceso de {{ -hud-skill-axe_unlock_title }}
hud-skill-axe_leap_knockback = Aumenta el retroceso del salto en un { $boost } %{ $SP }
hud-skill-axe_leap_cost_title = Coste de {{ -hud-skill-axe_unlock_title }}
hud-skill-axe_leap_cost = Reduce el coste del salto en un { $boost } %{ $SP }

# Cetro - Árbol de habilidades
## Drenar vida
hud-skill-sc_lifesteal_title = Drenar vida
hud-skill-sc_lifesteal = Lanza un rayo que absorbe la esencia vital de los enemigos
hud-skill-sc_lifesteal_damage_title = Daño
hud-skill-sc_lifesteal_damage = El rayo hace un { $boost } % más de daño{ $SP }
hud-skill-sc_lifesteal_regen_title = Regeneración de aguante
hud-skill-sc_lifesteal_regen = Recupera un { $boost } % de aguante adicional{ $SP }
hud-skill-sc_lifesteal_range_title = Alcance
hud-skill-sc_lifesteal_range = El rayo llega un { $boost } % más lejos{ $SP }
hud-skill-sc_lifesteal_lifesteal_title = Robo de vida
hud-skill-sc_lifesteal_lifesteal = Convierte un { $boost } % adicional del daño infligido en salud{ $SP }

## Campo de vida
hud-skill-sc_heal_title = Campo vital
hud-skill-sc_heal = Emana de ti un aura curativa que usa la esencia vital absorbida
hud-skill-sc_heal_heal_title = Potencia de {{ hud-skill-sc_heal_title }}
hud-skill-sc_heal_heal = Aumenta la curación que haces en un { $boost } %{ $SP }
hud-skill-sc_heal_cost_title = Coste de {{ hud-skill-sc_heal_title }}
hud-skill-sc_heal_cost = Curar consume un { $boost } % menos de aguante{ $SP }
hud-skill-sc_heal_duration_title = Duración de {{ hud-skill-sc_heal_title }}
hud-skill-sc_heal_duration = Los efectos del aura duran un { $boost } % más{ $SP }
hud-skill-sc_heal_range_title = Alcance de {{ hud-skill-sc_heal_title }}
hud-skill-sc_heal_range = El aura llega un { $boost } % más lejos{ $SP }

## Aura de protección
-hud-skill-sc_wardaura_title = Aura del guardián
hud-skill-sc_wardaura_unlock_title = Desbloquear {{ -hud-skill-sc_wardaura_title }}
hud-skill-sc_wardaura_unlock = Emana de ti un aura que te protege a ti y a tus aliados{ $SP }
hud-skill-sc_wardaura_strength_title = Potencia de {{ -hud-skill-sc_wardaura_title }}
hud-skill-sc_wardaura_strength = La potencia de la protección aumenta en un { $boost } %{ $SP }
hud-skill-sc_wardaura_duration_title = Duración de {{ -hud-skill-sc_wardaura_title }}
hud-skill-sc_wardaura_duration = Los efectos de la protección duran un { $boost } % más{ $SP }
hud-skill-sc_wardaura_range_title = Alcance de {{ -hud-skill-sc_wardaura_title }}
hud-skill-sc_wardaura_range = El aura llega un { $boost } % más lejos{ $SP }
hud-skill-sc_wardaura_cost_title = Coste de aguante de {{ -hud-skill-sc_wardaura_title }}
hud-skill-sc_wardaura_cost = El aura requiere un { $boost } % menos de aguante{ $SP }

# Árco - Árbol de habilidades
## Tiro de arco
hud-skill-bow_charged_title = Tiro de arco
hud-skill-bow_charged = Tensa tu arco para disparar una flecha
hud-skill-bow_charged_damage_title = Daño de {{ hud-skill-bow_charged_title }}
hud-skill-bow_charged_damage = Aumenta el daño infligido en un { $boost } %{ $SP }
hud-skill-bow_charged_speed_title = Velocidad de {{ hud-skill-bow_charged_title }}
hud-skill-bow_charged_speed = Aumenta la velocidad a la que tensas el arco en un { $boost } %{ $SP }
hud-skill-bow_charged_knockback_title = Retroceso de {{ hud-skill-bow_charged_title }}
hud-skill-bow_charged_knockback = Las flechas hacen retroceder a los enemigos un { $boost } % más{ $SP }

## Metralleta
hud-skill-bow_repeater_title = Metralleta
hud-skill-bow_repeater = Dispara una serie de flechas que van aumentando de velocidad
hud-skill-bow_repeater_damage_title = Daño de {{ hud-skill-bow_repeater_title }}
hud-skill-bow_repeater_damage = Aumenta el daño infligido en un { $boost } %{ $SP }
hud-skill-bow_repeater_cost_title = Coste de {{ hud-skill-bow_repeater_title }}
hud-skill-bow_repeater_cost = Reduce el coste de aguante al empezar una ráfaga en un { $boost } %{ $SP }
hud-skill-bow_repeater_speed_title = Velocidad de {{ hud-skill-bow_repeater_title }}
hud-skill-bow_repeater_speed = Aumenta la velocidad a la que se disparan flechas en un { $boost } %{ $SP }

## Escopeta
-hud-skill-bow_shotgun_title = Escopeta
hud-skill-bow_shotgun_unlock_title = Desbloquear Escopeta
hud-skill-bow_shotgun_unlock = Desbloquea la capacidad de disparar una multitud de flechas al mismo tiempo{ $SP }
hud-skill-bow_shotgun_damage_title = Daño de {{ -hud-skill-bow_shotgun_title }}
hud-skill-bow_shotgun_damage = Aumenta el daño infligido en un { $boost } %{ $SP }
hud-skill-bow_shotgun_spread_title = Dispersión de {{ -hud-skill-bow_shotgun_title }}
hud-skill-bow_shotgun_spread = Reduce la dispersión de las flechas en un { $boost } %{ $SP }
hud-skill-bow_shotgun_cost_title = Coste de {{ -hud-skill-bow_shotgun_title }}
hud-skill-bow_shotgun_cost = Reduce el coste de escopeta en un { $boost } %{ $SP }
hud-skill-bow_shotgun_arrow_count_title = Flechas de {{ -hud-skill-bow_shotgun_title }}
hud-skill-bow_shotgun_arrow_count = Aumenta el número de flechas por disparo en { $boost }{ $SP }

## Velocidad de proyectil
hud-skill-bow_projectile_speed_title = Velocidad de proyectil
hud-skill-bow_projectile_speed = Las flechas llegan más lejos al viajar un { $boost } % más rápido{ $SP }

# Bastón de fuego - Árbol de habilidades
## Bola de fuego
hud-skill-st_fireball_title = Bola de Fuego
hud-skill-st_fireball = Dispara una bola de fuego que explota al impactar
hud-skill-st_damage_title = Daño de {{ hud-skill-st_fireball_title }}
hud-skill-st_damage = Aumenta el daño infligido en un { $boost } %{ $SP }
hud-skill-st_explosion_radius_title = Radio de explosión de {{ hud-skill-st_fireball_title }}
hud-skill-st_explosion_radius = Aumenta el alcance de la explosión en un { $boost } %{ $SP }
hud-skill-st_energy_regen_title = Ganancia de aguante de {{ hud-skill-st_fireball_title }}
hud-skill-st_energy_regen = Aumenta la ganancia de aguante en un { $boost } %{ $SP }

## Lanzallamas
hud-skill-st_flamethrower_title = Lanzallamas
hud-skill-st_flamethrower = Lanza fuego, ¡fríelos a todos!
hud-skill-st_flamethrower_damage_title = Daño de {{ hud-skill-st_flamethrower_title }}
hud-skill-st_flamethrower_damage = Aumenta el daño infligido en un { $boost } %{ $SP }
hud-skill-st_flame_velocity_title = Velocidad de {{ hud-skill-st_flamethrower_title }}
hud-skill-st_flame_velocity = El fuego viaja un { $boost } % más rápido{ $SP }
hud-skill-st_energy_drain_title = Consumo de aguante de {{ hud-skill-st_flamethrower_title }}
hud-skill-st_energy_drain = El aguante se reduce un { $boost } % más lento{ $SP }
hud-skill-st_flamethrower_range_title = Alcance de {{ hud-skill-st_flamethrower_title }}
hud-skill-st_flamethrower_range = Las llamas llegan un { $boost } % más lejos{ $SP }

## Onda de choque
-hud-skill-st_shockwave_title = Onda de choque
hud-skill-st_shockwave_unlock_title = Desbloquear {{ -hud-skill-st_shockwave_title }}
hud-skill-st_shockwave_unlock = Desbloquea la habilidad de lanzar por los aires a los enemigos usando fuego{ $SP }
hud-skill-st_shockwave_damage_title = Daño de {{ -hud-skill-st_shockwave_title }}
hud-skill-st_shockwave_damage = Aumenta el daño infligido en un { $boost } %{ $SP }
hud-skill-st_shockwave_range_title = Alcance de {{ -hud-skill-st_shockwave_title }}
hud-skill-st_shockwave_range = Aumenta el alcance de la onda en un { $boost } %{ $SP }
hud-skill-st_shockwave_knockback_title = Retroceso de {{ -hud-skill-st_shockwave_title }}
hud-skill-st_shockwave_knockback = Aumenta la potencia de lanzamiento en un { $boost } %{ $SP }
hud-skill-st_shockwave_cost_title = Coste de {{ -hud-skill-st_shockwave_title }}
hud-skill-st_shockwave_cost = Reduce el coste de aguante en un { $boost } %{ $SP }

# Minería - Árbol de habilidades
hud-skill-mining_title = Minería
hud-skill-pick_strike_title = Picar
hud-skill-pick_strike = Pica rocas con el pico para conseguir minerales, gemas y experiencia
hud-skill-pick_strike_speed_title = Velocidad de {{ hud-skill-pick_strike_title }}
hud-skill-pick_strike_speed = Pica rocas más rápido{ $SP }
hud-skill-pick_strike_oregain_title = Producción de minerales de {{ hud-skill-pick_strike_title }}
hud-skill-pick_strike_oregain = Concede un { $boost } % de probabilidad de conseguir minerales adicionales.{ $SP }
hud-skill-pick_strike_gemgain_title = Producción de gemas de {{ hud-skill-pick_strike_title }}
hud-skill-pick_strike_gemgain = Concede un { $boost } % de probabilidad de conseguir gemas adicionales.{ $SP }
