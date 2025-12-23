# Descriptions and Help

command-help-template = { $usage } { $description }
command-help-list =
  { $client-commands }
  { $server-commands }

  Additionally, you can use the following shortcuts:
  { $additional-shortcuts }

## Server Commands

command-adminify-desc = Temporarily gives a player a restricted admin role or removes the current one (if not given)
command-airship-desc = Spawns an airship
command-alias-desc = Change your alias
command-area_add-desc = Adds a new build area
command-area_list-desc = List all build areas
command-area_remove-desc = Removes specified build area
command-aura-desc = Create an aura
command-body-desc = Change your body to different species
command-set_body_type-desc = Set your body type, Female or Male.
command-set_body_type-not_found = That's not a valid body type.
  Try one of:
  { $options }
command-set_body_type-no_body = Couldn't set body type as the target doesn't have a body.
command-set_body_type-not_character = Can only permanently set body type if the target is a player online as a character.
command-buff-desc = Cast a buff on player
command-build-desc = Toggles build mode on and off
command-ban-desc = Ban a player with a given username, for a given duration (if provided). Pass true for overwrite to alter an existing ban.
command-ban-ip-desc = Ban a player with a given username, for a given duration (if provided). Unlike the normal ban this also additionally bans the IP-address associated with this user. Pass true for overwrite to alter an existing ban.
command-battlemode-desc = Set your battle mode to:
  + pvp (player vs player)
  + pve (player vs environment).
  If called without arguments will show current battle mode.
command-battlemode_force-desc = Change your battle mode flag without any checks
command-campfire-desc = Spawns a campfire
command-clear_persisted_terrain-desc = Clears nearby persisted terrain
command-create_location-desc = Create a location at the current position
command-death_effect-dest = Adds an on-death effect to the target entity
command-debug_column-desc = Prints some debug information about a column
command-debug_ways-desc = Prints some debug information about a column's ways
command-delete_location-desc = Delete a location
command-destroy_tethers-desc = Destroy all tethers connected to you
command-disconnect_all_players-desc = Disconnects all players from the server
command-dismount-desc = Dismount if you are riding, or dismount anything riding you
command-dropall-desc = Drops all your items on the ground
command-dummy-desc = Spawns a training dummy
command-explosion-desc = Explodes the ground around you
command-faction-desc = Send messages to your faction
command-give_item-desc = Give yourself some items. For an example or to auto complete use Tab.
command-gizmos-desc = Manage gizmo subscriptions.
command-gizmos_range-desc = Change the range of gizmo subscriptions.
command-goto-desc = Teleport to a position
command-goto-rand = Teleport to a random position
command-group-desc = Send messages to your group
command-group_invite-desc = Invite a player to join a group
command-group_kick-desc = Remove a player from a group
command-group_leave-desc = Leave the current group
command-group_promote-desc = Promote a player to group leader
command-health-desc = Set your current health
command-into_npc-desc = Convert yourself to an NPC. Be careful!
command-join_faction-desc = Join/leave the specified faction
command-jump-desc = Offset your current position
command-kick-desc = Kick a player with a given username
command-kill-desc = Kill yourself
command-kill_npcs-desc = Kill the NPCs
command-kit-desc = Place a set of items into your inventory.
command-lantern-desc = Change your lantern's strength and color
command-light-desc = Spawn entity with light
command-lightning-desc = Lightning strike at current position
command-location-desc = Teleport to a location
command-make_block-desc = Make a block at your location with a color
command-make_npc-desc = Spawn entity from config near you.
  For an example or to auto complete use Tab.
command-make_sprite-desc = Make a sprite at your location, to define sprite attributes use ron syntax for a StructureSprite.
command-make_volume-desc = Create a volume (experimental)
command-motd-desc = View the server description
command-mount-desc = Mount an entity
command-object-desc = Spawn an object
command-outcome-desc = Create an outcome
command-permit_build-desc = Grants player a bounded box they can build in
command-players-desc = Lists players currently online
command-poise-desc = Set your current poise
command-portal-desc = Spawns a portal
command-region-desc = Send messages to everyone in your region of the world
command-reload_chunks-desc = Reloads chunks loaded on the server
command-remove_lights-desc = Removes all lights spawned by players
command-repair_equipment-desc = Repairs all equipped items
command-reset_recipes-desc = Resets your recipe book
command-respawn-desc = Teleport to your waypoint
command-revoke_build-desc = Revokes build area permission for player
command-revoke_build_all-desc = Revokes all build area permissions for player
command-safezone-desc = Creates a safezone
command-say-desc = Send messages to everyone within shouting distance
command-scale-desc = Scale your character
command-server_physics-desc = Set/unset server-authoritative physics for an account
command-set_motd-desc = Set the server description
command-set-waypoint-desc = Set your waypoint to your current location.
command-ship-desc = Spawns a ship
command-site-desc = Teleport to a site
command-skill_point-desc = Give yourself skill points for a particular skill tree
command-skill_preset-desc = Gives your character desired skills.
command-spawn-desc = Spawn a test entity
command-spot-desc = Find and teleport to the closest spot of a certain kind.
command-sudo-desc = Run command as if you were another entity
command-tell-desc = Send a message to another player
command-tether-desc = Tether another entity to yourself
command-time-desc = Set the time of day
command-time_scale-desc = Set scaling of delta time
command-tp-desc = Teleport to another entity
command-rtsim_chunk-desc = Display information about the current chunk from rtsim
command-rtsim_info-desc = Display information about an rtsim NPC
command-rtsim_npc-desc = List rtsim NPCs that fit a given query (e.g: simulated,merchant) in order of distance
command-rtsim_purge-desc = Purge rtsim data on next startup
command-rtsim_tp-desc = Teleport to an rtsim npc
command-unban-desc = Remove the ban for the given username. If there is an linked IP ban it will be removed as well.
command-unban-ip-desc = Remove just the IP ban for the given username.
command-version-desc = Prints server version
command-weather_zone-desc = Create a weather zone
command-whitelist-desc = Adds/removes username to whitelist
command-wiring-desc = Create wiring element
command-world-desc = Send messages to everyone on the server
command-wiki-desc = Open the wiki or search for a topic
command-reset_tutorial-desc = Reset the in-game tutorial to its starting state
command-reset_tutorial-success = Reset tutorial state.
command-naga-desc = Toogle use of naga in initial shader processing (not persisted)
# Command: /players
players-list-header = { $count ->
  [1] { $count } player online
    { $player_list }
  *[other] { $count } players online
    { $player_list }
}
## Voxygen Client Commands

command-clear-desc = Clears all messages in chat. Affects all chat tabs.
command-experimental_shader-desc = Toggles an experimental shader.
command-help-desc = Display information about commands
command-mute-desc = Mutes chat messages from a player.
command-unmute-desc = Unmutes a player muted with the 'mute' command.
command-waypoint-desc = Show the location of the current waypoint
command-preprocess-target-error = Expected { $expected_list } after '@' found { $target }
command-preprocess-not-looking-at-valid-target = Not looking at a valid target
command-preprocess-not-selected-valid-target = Not selecting a valid target
command-preprocess-not-valid-viewpoint-entity = Not viewing from a valid viewpoint entity
command-preprocess-not-riding-valid-entity = Not riding a valid entity
command-preprocess-not-valid-rider = No valid rider
command-preprocess-no-player-entity = No player entity
command-invalid-command-message =
  Could not find a command named { $invalid-command }.
  Did you mean any of the following?
  { $most-similar-command }
  { $commands-with-same-prefix }

  Type /help to see a list of all commands.
command-mute-cannot-mute-self = You cannot mute yourself
command-mute-success = Successfully muted { $player }
command-mute-no-player-found = Could not find a player named { $player }
command-mute-already-muted = { $player } is already muted
command-mute-no-player-specified = You must specify a player
command-unmute-cannot-unmute-self = You cannot unmute yourself
command-unmute-success = Successfully unmuted { $player }
command-unmute-no-muted-player-found = Could not find a muted player named { $player }
command-unmute-no-player-specified = You must specify a player to mute
command-shader-backend = Current Shader Backend: { $shader-backend }
# Only returns a list of shaders
command-experimental-shaders-list = { $shader-list }
command-experimental-shaders-not-found = There are no experimental shaders
command-experimental-shaders-enabled = Enabled { $shader }
command-experimental-shaders-disabled = Disabled { $shader }
command-experimental-shaders-not-a-shader = { $shader } is not an expermimental shader, use this command with any arguments to see a complete list.
command-experimental-shaders-not-valid = You must specify a valid experimental shader, to get a list of experimental shaders, use this command without any arguments.

# Results and Warning

command-no-permission = You don't have permission to use '/{ $command_name }'
command-position-unavailable = Cannot get position for { $target }
command-player-role-unavailable = Cannot get administrator roles for { $target }
command-uid-unavailable = Cannot get UID for { $target }
command-area-not-found = Could not find area named '{ $area }'
command-player-not-found = Player '{ $player }' not found!
command-player-uuid-not-found = Player with UUID '{ $uuid }' not found!
command-username-uuid-unavailable = Unable to determine UUID for username { $username }
command-uuid-username-unavailable = Unable to determine username for UUID  { $uuid }
command-no-sudo = It's rude to impersonate people
command-entity-dead = Entity '{ $entity }' is dead!
command-error-write-settings = Failed to write settings file to disk, but succeeded in memory.
  Error (storage): { $error }
  Success (memory): { $message }
command-error-while-evaluating-request = Encountered an error while validating the request: { $error }
command-give-inventory-full = Player inventory full. Gave { $given ->
  [1] only one
  *[other] { $given }
} of { $total } items.
command-give-inventory-success = Added { $total } x { $item } to the inventory.
command-invalid-item = Invalid item: { $item }
command-invalid-block-kind = Invalid block kind: { $kind }
command-nof-entities-at-least = Number of entities should be at least 1
command-nof-entities-less-than = Number of entities should be less than 50
command-entity-load-failed = Failed to load entity config: { $config }
command-spawned-entities-config = Spawned { $n } entities from config: { $config }
command-invalid-sprite = Invalid sprite kind: { $kind }
command-time-parse-too-large = { $n } is invalid, cannot be larger than 16 digits.
command-time-parse-negative = { $n } is invalid, cannot be negative.
command-time-backwards = { $t } is before the current time, time cannot go backwards.
command-time-invalid = { $t } is not a valid time.
command-time-current = It is { $t }
command-time-unknown = Time unknown
command-rtsim-purge-perms = You must be a real admin (not just a temporary admin) to purge rtsim data.
command-chunk-not-loaded = Chunk { $x }, { $y } not loaded
command-chunk-out-of-bounds = Chunk { $x }, { $y } not within map bounds
command-spawned-entity = Spawned entity with ID: { $id }
command-spawned-dummy = Spawned a training dummy
command-spawned-airship = Spawned an airship
command-spawned-campfire = Spawned a campfire
command-spawned-safezone = Spawned a safe zone
command-volume-size-incorrect = Size has to be between 1 and 127.
command-volume-created = Created a volume
command-permit-build-given = You are now permitted to build in '{ $area }'
command-permit-build-granted = Permission to build in '{ $area }' granted
command-revoke-build-recv = Your permission to build in '{ $area }' has been revoked
command-revoke-build = Permission to build in '{ $area }' revoked
command-revoke-build-all = Your build permissions have been revoked.
command-revoked-all-build = All build permissions revoked.
command-no-buid-perms = You do not have permission to build.
command-set-build-mode-off = Toggled build mode off.
command-set-build-mode-on-persistent = Toggled build mode on. Experimental terrain persistence is enabled. The server will attempt to persist changes, but this is not guaranteed.
command-set-build-mode-on-unpersistent = Toggled build mode on. Changes will not be persisted when a chunk unloads.
command-set_motd-message-added = Server message of the day set to { $message }
command-set_motd-message-removed = Removed server message of the day
command-set_motd-message-not-set = This locale had no motd set
command-set-waypoint-result = Waypoint set!
command-invalid-alignment = Invalid alignment: { $alignment }
command-kit-not-enough-slots = Inventory doesn't have enough slots
command-lantern-unequiped = Please equip a lantern first
command-lantern-adjusted-strength = You adjusted flame strength.
command-lantern-adjusted-strength-color = You adjusted flame strength and color.
command-explosion-power-too-high = Explosion power mustn't be more than { $power }
command-explosion-power-too-low = Explosion power must be more than { $power }
# Note: Do not translate "confirm" here
command-disconnectall-confirm = Please run the command again with the second argument of "confirm" to confirm that
  you really want to disconnect all players from the server
command-invalid-skill-group = { $group } is not a skill group!
command-unknown = Unknown command
command-disabled-by-settings = Command disabled in server settings
command-battlemode-intown = You need to be in town to change battle mode!
command-battlemode-cooldown = Cooldown period active. Try again in { $cooldown } seconds
command-battlemode-available-modes = Available modes: pvp, pve
command-battlemode-same = Attempted to set the same battle mode
command-battlemode-updated = New battle mode: { $battlemode }
command-buff-unknown = Unknown buff: { $buff }
command-buff-data = Buff argument '{ $buff }' requires additional data
command-buff-body-unknown = Unknown body spec: { $spec }
command-skillpreset-load-error = Error while loading presets
command-skillpreset-broken = Skill preset is broken
command-skillpreset-missing = Preset does not exist: { $preset }
command-location-invalid = Location name '{ $location }' is invalid. Names may only contain lowercase ASCII and
  underscores
command-location-duplicate = Location '{ $location }' already exists, consider deleting it first
command-location-not-found = Location '{ $location }' does not exist
command-location-created = Created location '{ $location }'
command-location-deleted = Deleted location '{ $location }'
command-locations-empty = No locations currently exist
command-locations-list = Available locations: { $locations }
# Note: Do not translate these weather names
command-weather-valid-values = Valid values are 'clear', 'rain', 'wind' and 'storm'.
command-scale-set = Set scale to { $scale }
command-repaired-items = Repaired all equipped items
command-repaired-inventory_items = Repaired all items
command-message-group-missing = You are using group chat but do not belong to a group. Use /world or
  /region to change chat.
command-tell-to-yourself = You can't /tell yourself.
command-transform-invalid-presence = Cannot transform in the current presence
command-aura-invalid-buff-parameters = Invalid buff parameters for aura
command-aura-spawn = Spawned new aura attached to entity
command-aura-spawn-new-entity = Spawned new aura
command-reloaded-chunks = Reloaded { $reloaded } chunks
command-server-no-experimental-terrain-persistence = Server was compiled without terrain persistence enabled
command-experimental-terrain-persistence-disabled = Experimental terrain persistence is disabled
command-adminify-assign-higher-than-own = Cannot assign someone a temporary role higher than your own permanent one.
command-adminify-reassign-to-above = Cannot reassign a role for anyone with your role or higher.
command-adminify-cannot-find-player = Cannot find player entity!
command-adminify-already-has-role = Player already has that role!
command-adminify-already-has-no-role = Player already has no role!
command-adminify-role-downgraded = Role for player { $player } downgraded to { $role }
command-adminify-role-upgraded = Role for player { $player } upgraded to { $role }
command-adminify-removed-role = Role removed from player { $player }: { $role }
command-ban-added = Added { $player } to the banlist with reason: { $reason }
command-ban-already-added = { $player } is already on the banlist
command-ban-ip-added = Added { $player } to the regular banlist and IP banlist with reason: { $reason }
command-ban-ip-queued = Added { $player } to the regular banlist and queued an IP ban with reason: { $reason }
command-faction-join = Please join a faction with /join_faction
command-group-join = Please create a group first
command-group_invite-invited-to-group = Invited { $player } to the group.
command-group_invite-invited-to-your-group = { $player } has been invited to your group.
command-into_npc-warning = I hope you aren't abusing this!
command-kick-higher-role = Cannot kick players with roles higher than your own.
command-respawn-no-waypoint = No waypoint set
command-site-not-found = Site not found
command-sudo-higher-role = Cannot sudo players with roles higher than your own.
command-sudo-no-permission-for-non-players = You don't have permission to sudo non-players.
command-time_scale-current = The current time scale is { $scale }.
command-time_scale-changed = Set time scale to { $scale }.
command-unban-successful = { $player } was successfully unbanned.
command-unban-ip-successful = The IP banned via user "{ $player }" was successfully unbanned (this user will remain banned)
command-unban-already-unbanned = { $player } was already unbanned.
command-version-current = Server is running { $version }
command-whitelist-added = Added to whitelist: { $username }
command-whitelist-already-added = Already in whitelist: { $username }!
command-whitelist-removed = Removed from whitelist: { $username }
command-whitelist-unlisted = Not part of whitelist: { $username }
command-whitelist-permission-denied = Permission denied to remove user: { $username }
command-outcome-variant_expected = Outcome variant expected
command-outcome-expected_body_arg = Expected body argument
command-outcome-expected_entity_arg = Expected entity argument
command-outcome-expected_skill_group_kind = Expected valid ron SkillGroupKind
command-outcome-expected_frontent_specifier = Expected frontent specifier
command-outcome-expected_integer = Expected integer
command-outcome-expected_sprite_kind = Expected SpriteKind
command-outcome-invalid_outcome = { $outcome } is not a valid outcome
command-death_effect-unknown = Unknown death effect { $effect }.
command-spot-spot_not_found = Didn't find any spots of that kind in this world.
command-spot-world_feature = The `worldgen` feature has to be enabled to run this command.
command-cannot-send-message-hidden = Cannot send messages as a hidden spectator.
command-destroyed-tethers = All tethers destroyed! You are now free
command-destroyed-no-tethers = You're not connected to any tethers
command-dismounted = Dismounted
command-no-dismount = You're not riding or being ridden
command-client-has-no-socketaddr = Cannot get socker addr (connected via mpsc connection) for { $target }
command-parse-duration-error = Could not parse duration: { $error }
command-waypoint-result = Your current waypoint is at { $waypoint };
command-waypoint-error = Could not find your waypoint.

# Unreachable/untestable but added for consistency

command-player-info-unavailable = Cannot get player information for { $target }
command-unimplemented-spawn-special = Spawning special entities is not implemented
command-kit-inventory-unavailable = Could not get inventory
command-inventory-cant-fit-item = Can't fit item to inventory
# Emitted by /disconnect_all when you don't exist (?)
command-you-dont-exist = You do not exist, so you cannot use this command
command-entity-has-no-client = Player has no client client component: { $target }
