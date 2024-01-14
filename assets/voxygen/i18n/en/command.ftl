command-no-permission = You don't have permisison to use '/{ $command_name }'
command-position-unavailable = Cannot get position for { $target }
command-player-role-unavailable = Cannot get administrator roles for { $target }
command-uid-unavailable = Cannot get uid for { $target }
command-area-not-found = Could not find area named '{ $area }'
command-player-not-found = Player '{ $player }' not found!
command-player-uuid-not-found = Player with UUID '{ $uuid }' not found!
command-username-uuid-unavailable = Unable to determine UUID for username { $username }
command-uuid-username-unavailable = Unable to determine username for UUID  { $uuid }
command-no-sudo = It's rude to impersonate people
command-entity-dead = Entity '{ $entity }' is dead!
command-error-while-evaluating-request = Encountered an error while validating the request: { $error }
command-give-inventory-full = Player inventory full. Gave { $given ->
  [1] only one
  *[other] { $given }
} of { $total } items.
command-invalid-item = Invalid item: { $item }
command-invalid-block-kind = Invalid block kind: { $kind }
command-nof-entities-at-least = Number of entities should be at least 1
command-nof-entities-less-than = Number of entities should be less than 50
command-entity-load-failed = Failed to load entity config: { $config }
command-spawned-entities-config = Spawned { $n } entities from config: { $config }
command-invalid-sprite = Invalid sprite kind: { $kind }
command-time-parse-too-large = { $n } is invalid, cannot be larger than 16 digits
command-time-parse-negative = { $n } is invalid, cannot be negative.
command-time-backwards = { $t } is before the current time, time cannot go backwards.
command-time-invalid = { $t } is not a valid time.
command-rtsim-purge-perms = You must be a real admin (not just a temporary admin) to purge rtsim data.
command-chunk-not-loaded = Chunk { $x }, { $y } not loaded
command-chunk-out-of-bounds = Chunk { $x }, { $y } not within map bounds
command-spawned-entity = Spawned entity with ID: { $id }
command-spawned-dummy = Spawned a training dummy
command-spawned-airship = Spawned an airship
command-spawned-campfire = Spawned a campfire
command-spawned-safezone = Spawned a safezone
command-volume-size-incorrect = Size has to be between 1 and 127.
command-volume-created = Created a volume
command-permit-build-given = You are now permitted to build in '{ $area }'
command-permit-build-granted = Permission to build in '{ $area }' granted
command-revoke-build-recv = Your permission to build in '{ $area }' has been revoked
command-revoke-build = Permission to build in '{ $area }' revoked
command-revoke-build-all = Your build permissions have been revoked.
command-revoked-all-build = All build permissions revoked
command-no-buid-perms = You do not have permission to build.
command-set-build-mode-off = Toggled build mode off.
command-set-build-mode-on-persistent = Toggled build mode on. Experimental terrain persistence is enabled. The server will attempt to persist changes, but this is not guaranteed 
command-set-build-mode-on-unpersistent = Toggled build mode on. Changes will not be persisted when a chunk unloads.
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
command-battlemode-same = Attempted to set the same battlemode
command-battlemode-updated = New battlemode: { $battlemode }
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
command-weather-valid-values = Valid values are 'clear', 'rain', 'wind', 'storm'
command-scale-set = Set scale to { $scale }
command-repaired-items = Repaired all equipped items
command-message-group-missing = You are using group chat but do not belong to a group. Use /world or
  /region to change chat.
command-tell-request = { $sender } wants to talk to you.

# Unreachable/untestable but added for consistency

command-player-info-unavailable = Cannot get player information for { $target }
command-unimplemented-waypoint-spawn = Waypoint spawning is not implemented
command-unimplemented-teleporter-spawn = Teleporter spawning is not implemented
command-kit-inventory-unavailable = Could not get inventory
command-inventory-cant-fit-item = Can't fit item to inventory
# Emitted by /disconnect_all when you dont exist (?)
command-you-dont-exist = You do not exist, so you cannot use this command
command-destroyed-tethers = All tethers destroyed! You are now free
command-destroyed-no-tethers = You're not connected to any tethers
command-dismounted = Dismounted
command-no-dismount = You're not riding or being ridden
