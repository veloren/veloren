# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added chat commands for inviting, kicking, leaving, and promoting in groups
- Aura system
- Campfire resting heal
- Initial support for game plugins, both server-side and client-side
- Reflective LoD water
- Map indicators for group members
- Hot-reloading for i18n, sounds, loot lotteries, and more
- Initial support for alternate style keyboards
- Flying birds travel the world
- Plugin system now based on Wasmer 1.0.0
- Added 4x Bag loadout slots, used for upgrading inventory space
- Added an additional Ring loadout slot
- The inventory can now be expanded to fill the whole window
- Added /dropall admin command (drops all inventory items on the ground)
- Skill trees
- Lactose tolerant golems
- 6 different gems. (Topaz, Amethyst, Sapphire, Emerald, Ruby and Diamond)

### Changed

- Doubled range of ScaleMode slider when set to Custom
- Glider can now be deployed mid-air at the cost of some stamina based on fall speed
- Translations are now folders with multiple files instead of a huge single file
- Default inventory slots reduced to 18 - existing characters given 3x 6-slot bags as compensation
- Protection rating was moved to the top left of the loadout view 
- Changed camera smoothing to be off by default.
- Fixed AI behavior so only humanoids will attempt to roll
- Footstep SFX is now dependant on distance moved, not time since last play
- Adjusted most NPCs hitboxes to better fit their models.
- Changed crafting recipes involving shiny gems to use diamonds instead.
- Cave scatter now includes all 6 gems.

### Removed

- SSAAx4 option
- The Stats button and associated screen were removed
- Levels
- Shiny Gems (replaced with diamonds)

### Fixed

- Fixed a bug that would cause a server crash when a player levelled up or fired
  a projectile in very specific circumstances
- Fixed a bug where buff/debuff UI elements would flicker when you had more than
  one of them active at the same time
- Made zooming work on wayland

## [0.8.0] - 2020-11-28

### Added

- New level of detail feature, letting you see all the world's terrain at any view distance.
- Point and directional lights now cast realistic shadows, using shadow mapping.
- Added leaf and chimney particles
- Some more combat sound effects
- Beehives and bees
- Fireflies
- Fullscreen modes now show two options (exclusive and borderless)
- Added banlist and `/ban`, `/unban`, and `/kick` commands for admins
- A new dungeon boss (venture there and discover it yourself)
- Adaptive stride setup for more dynamic run behavior
- Theropod body
- Several new animals
- Item quality indicators
- Added a jump/burst attack for the bow to the skillbar
- Gave the axe a third attack
- A new secondary charged melee attack for the hammer
- Added Dutch translations
- Buff system
- Sneaking lets you be closer to enemies without being detected
- Flight
- Roll dodges melee attacks, and reduces the height of your hitbox
- Persistent waypoints (start from the last camp fire you visited)
- NPCs use all three weapon skills in combat
- Speed stat to weapons which affects weapon attack speed
- Saving of the last selected character in the character selection screen
- Autoselecting the newly created character
- Deselecting when the selected character is deleted
- Upscaling support
- Added "Persist Combo from Combo Melee State" when rolling mid-combo
- You can no longer spam hammer and bow special when stamina is 0
- Biome and site specific music system
- Ambient SFX emitted from terrain blocks
- Campfire SFX
- Wind SFX system
- Added Norwegian language
- Roll can now interrupt attacks
- Birch forests
- Willow forests
- More significant temperature variation across the world
- Initial implementation of real-time world simulation
- Travellers that explore the world
- HDR rendering
- Map site icons
- Map panning
- Innumerable minor improvements to world generation
- Variable dungeon difficulty
- Aurora Borealis (localised entirely within the kitchen)
- Block-based voxel lighting
- Animals now have customized attacks and AI

### Changed

- The world map has been refactored to support arbitrary sizes and compute horizon maps.
- Veloren's lighting has been completely overhauled.
- The graphics options were made much more flexible and configurable.
- Many shader optimizations.
- Voxel model creation was switched to use greedy meshing, improving performance.
- Animation and terrain math were switched to use SIMD where possible, improving performance.
- The way we cache glyphs was refactored, fixed, and optimized.
- Colors for models and figures were adjusted to account for the saturation hack.
- Overhauled world colours
- Improved projectile physics
- Improved overhead aiming
- Improved first person aiming
- Figure meshing no longer blocks the main thread.
- Overhauled persistence layer including no longer storing serialized JSON items in the database
- Overhauled representation of blocks to permit fluid and sprite coexistence
- Overhauled sword
- Reworked healing sceptre
- Split out the sections of the server settings that can be edited and saved by the server.
- Revamped structure of where settings, logs, and game saves are stored so that almost everything is in one place.
- Moved hammer leap attack to skillbar
- Reworked fire staff
- Overhauled cloud shaders to add mist, light attenuation, an approximation of rayleigh scattering, etc.
- Allowed collecting nearby blocks without aiming at them
- Made voxygen wait until singleplayer server is initialized before attempting to connect, removing the chance for it to give up on connecting if the server takes a while to start
- Log where userdata folder is located
- Switched to a Whittaker map for better tree spawning patterns
- Switched to procedural snow cover on trees
- Significantly improved terrain generation performance
- Significantly stabilized the game clock, to produce more "constant" TPS
- Transitioned main menu and character selection screen to a using iced for the ui (fixes paste keybinding on macos, removes password field limits, adds tabbing between input fields in the main menu, adds language selection in the main menu)
- Made settings less likely to reset when the format changes
- Adjusted some keybindings
- Consumables can now trigger multiple effects and buffs
- Overhauled overworld spawns depending on chunk attributes
- Improved cloud and water shader quality

### Removed

- MSAA has been removed due to incompatibility with greedy meshing.
- Removed a saturation hack that led to colors being improperly displayed.

### Fixed

- Fixed a bug where leaving the Settings menu by pressing "N" in single player kept the game paused.
- Fixed a bug where the closest item would be picked up instead of a selected item.
- Fixed a bug where camera zoom in and zoom out distance didn't match.
- Fixed a bug where a nearby item would also be collected when collecting collectible blocks
- Fixed a bug where firing fast projectile at a downwards angle caused them to veer off at a higher angle
- Fixed a bug where ui scale in the login menu was not updated when changed in-game
- Fixed a bug which caused campfires and other stuff to duplicate
- Significantly improved water movement AI to stop entities getting stuck
- Prevented entities, sprites and particles being lit when not visible to the sun

## [0.7.0] - 2020-08-15

### Added

- Display item name over loot/dropped items
- Added Lottery system for loot
- Added context-sensitive crosshair
- Announce alias changes to all clients
- Dance animation
- Speech bubbles appear when nearby players talk
- NPCs call for help when attacked
- Eyebrows and shapes can now be selected
- Character name and level information to chat, social tab and `/players` command
- Added inventory, armour and weapon saving
- Show where screenshots are saved in the chat
- Added basic auto walk
- Added weapon/attack sound effects
- M2 attack for bow
- Hotbar persistence
- Alpha version Disclaimer
- Server whitelist
- Optional server-side maximum view distance
- MOTD on login
- Added group chat `/join_group` `/group`
- Added faction chat `/join_faction` `/faction`
- Added regional, local, and global chat (`/region`, `/say`, and `/world`, respectively)
- Added command shortcuts for each of the above chat modes (`/g`, `/f`, `/r`, `/s`, and `/w`, respectively and `/t` for `/tell`)
- Ability to wield 2 × 1h weapons and shields (Note: 1h weapons & shields are not currently avaliable, see [!1095](https://gitlab.com/veloren/veloren/-/merge_requests/1095) for more info)
- Zoomable Map
- M2 attack for hammer
- Spawnable training dummies
- New quadruped_low body for reptile-likes
- Added new animals
- Better pathfinding
- Bombs
- Training dummy items
- Added spin attack for axe
- Creature specific stats
- Minimap compass
- Initial crafting system implementation
- Protection stat to armor that reduces incoming damage
- Loading-Screen tips
- Feeding animation for some animals
- Power stat to weapons which affects weapon damage
- Add detection of entities under the cursor
- Functional group-system with exp-sharing and disabled damage to group members
- Some Campfire, fireball & bomb; particle, light & sound effects.
- Added firework recipe
- Added setting to change resolution
- Rare (unfinished) castles
- Caves with monsters and treasure
- Furniture and decals in towns

### Changed

- Improved camera aiming
- Made civsim, sites, etc. deterministic from the same seed
- Improved animations by adding orientation variation
- new tail bone for quad_small body
- slim the game size through lossless asset optimization
- Lanterns now stop glowing if you throw a lit one out of your inventory
- Fixed a crash caused by certain audio devices on OSX
- Bow animations now show held arrows
- Fixed a bug where walk/run sfx played while a character rolled/dodged
- Energy regen resets on last ability use instead of on wield
- Fixed unable to use ability; Secondary and ability3 (fire rod) will now automatically wield
- Gliding is now a toggle that can be triggered from the ground
- Replaced `log` with `tracing` in all crates
- Switch to a new network backend that will allow several improvements in the future
- Connection screen fails after 4 minutes if it can't connect to the server instead of 80 minutes
- Rebuilt quadruped_medium/quadruped_small animation and assets
- Disabled destruction of most blocks by explosions
- Disable damage to pets
- Made pets healable
- Rebalanced fire staff
- Animals are more effective in combat
- Pathfinding is much smoother and pets are cleverer
- Animals run/turn at different speeds
- Updated windowing library (winit 0.19 -> 0.22)
- Bow M2 is now a charged attack that scales the longer it's held
- Fixed window resizing on Mac OS X.
- Dehardcoded many item variants
- Tooltips avoid the mouse better and disappear when hovered
- Improved social window functions and visuals
- Changed agent behaviour to allow fleeing
- Waypoints now spawn on dungeon staircases

### Removed

- Wield requirement to swap loadout; fixes issue with unable swap loadout outside of combat
- Disclaimer wall of text on first startup

## [0.6.0] - 2020-05-16

### Added

- Added music system
- Added zoomable and rotatable minimap
- Added rotating orientation marker to main-map
- Added daily Mac builds
- Allow spawning individual pet species, not just generic body kinds
- Configurable fonts
- Configurable keybindings from the Controls menu
- Translation status tracking
- Added gamma setting
- Added new orc hairstyles
- Added SFX for wielding/unwielding weapons
- Fixed NPCs attacking the player forever after killing them
- Added SFX for collecting, dropping and using inventory items
- New attack animation
- Weapon control system
- Game pauses when in single player and pause menu
- Added authentication system (to play on the official server register on https://account.veloren.net)
- Added gamepad/controller support
- Added player feedback when attempting to pickup an item with a full inventory
- Added free look
- Added Italian translation
- Added Portuguese translation
- Added Turkish translation
- Added Traditional Chinese translation
- Complete rewrite of the combat system into a state machine
- Abilities like Dash and Triplestrike
- Armor can now be equipped as items
- Fireball explosions
- Inventory supports stacking
- Many new armors and weapons to find in chests
- Fleshed out "attack" animation into alpha, beta and spin type attacks
- Fleshed out range attack into charging and shooting animations for staff/bow
- Customized attack animation for hammers and axes
- Added German translation
- Added a silhouette for players when they are occluded
- Added transparency to the player when zooming in
- Made armor and hotbar slots actually function
- Added dragging and right-click to use functionality to inventory, armor & hotbar slots
- Added capes, lanterns, tabards, rings, helmets & necklaces as equippable armor
- 6 new music tracks
- Added basic world and civilization simulation
- Added overhauled towns
- Added fields, crops and scarecrows
- Added paths
- Added bridges
- Added procedural house generation
- Added lampposts
- Added NPCs that spawn in towns
- Added simple dungeons
- Added sub-voxel noise effect
- Added waypoints next to dungeons
- Made players spawn in towns
- Added non-uniform block heights
- Added `/sudo` command
- Added a Level of Detail (LoD) system for terrain sprites and entities
- Added owl, hyena, parrot, cockatrice, red dragon NPCs
- Added dungeon entrances
- Villagers tools and clothing
- Cultists clothing
- You can start the game by pressing "enter" from the character selection menu
- Added server-side character saving
- Player now starts with a lantern. Equipping/unequipping a lantern has the same effect as the `/lantern` command
- Added tab completion in chat for player names and chat commands
- Added server persistence for character stats
- Added a popup when setting your character's waypoint
- Added dungeon arenas
- Added dungeon bosses and rare boss loot
- Added 2 sets of armour. One Steel and one Leather.

### Changed

- The /give_item command can now specify the amount of items. Syntax is now `/give_item <name> [num]`
- Brighter / higher contrast main-map
- Removed highlighting of non-collectible sprites
- Fixed /give_exp ignoring player argument
- Extend run sfx to small animals to prevent sneak attacks by geese.
- Decreased clientside latency of ServerEvent mediated effects (e.g. projectiles, inventory operations, etc)
- Started changing the visual theme of the UI
- Merge of the Bag and Character-Screen
- Merge of the Map and Questlog
- Overhauled icon art
- Asset cleanup to lower client-size
- Rewrote the humanoid skeleton to be more ideal for attack animations
- Arrows can no longer hurt their owners
- Increased overall character scale
- `/sudo player /tp` is short for `/sudo player /tp me`
- The `/object` command can create any object in comp::object::Body
- The `/help` command takes an optional argument. `/help /sudo` will show you information about only the sudo command.

### Removed

## [0.5.0] - 2020-01-31

### Added

- Added new debug item
- Bows give experience by projectiles having an owner
- Allow cancelling chunk generation
- Include licence in assets
- Added dropping items
- Added initial region system implementation
- Added /giveitem command
- Strip Linux executables
- Added moon
- Added clouds
- Added tarpaulin coverage
- Added ability to jump while underwater
- Added proper SFX system
- Added changelog
- Added animated Map and Minimap position indicator
- Added visuals to indicate strength compared to the player
- Added Scrolling Combat Text (SCT) & Settings for it
- Added a Death Screen and Hurt Screen
- Added randomly selected Loading Screen background images
- Added options to disable clouds and to use cheaper water rendering
- Added client-side character saving
- Added a localization system to provide multi-language support
  to voxygen
- Added French language for Voxygen
- Added rivers and lakes which follow realistic physical paths.
- Added a sophisticated erosion system for world generation which
  dramatically changes the world layout.
- Added tracking of sediment vs. bedrock, which is visually reflected in the
  world.
- Added map saving and loading for altitude and bedrock, with built in
  versioning for forwards compatibility.
- Added a default map, which is used to speed up starting single player.
- Added a 3D renderered map, which is also used by the server to send the map
  to the client.
- Added fullscreen and window size to settings so that they can be persisted
- Added coverage based scaling for pixel art
- 28 new mobs
- Added waypoints
- Added pathfinding to NPCs
- Overhauled NPC AI
- Pets now attack enemies and defend their owners
- Added collars to tame wild animals

### Changed

- Controls pane in settings window now shows actual configured keys
- Fixed scroll wheel and roll keys on OS X
- Fixed near and far view planes
- Improvements to armor names
- Animation fixes to line up with true positions
- Proper message for command permission check failure
- Improved meshing
- Improved dusk
- Improved movement and climbing
- Improved water rendering and chunk render order
- Moved computations to terrain fragment shaders
- Fixed title music
- Made rolling less violent when changing directions
- Fixed single player crash
- Improved error information in client and server
- Store items as RON files
- Updated download info in readme
- Fixed cloud performance
- Fixed region display name
- Fixed the bow fire rate
- Healthbars now flash on critical health
- Fixed ghosts when going back to character screen
- Fixed not being able to unmount
- Fixed non-humanoids being able to climb and glide
- Made shadows and lights use interpolated positions
- Changed "Create Character" button position
- Made clouds bigger, more performant and prettier
- Terrain meshing optimized further
- Tree leaves no longer color blended
- Actual character stats displayed in character window
- Made significant changes to the noise functions used for world generation.
- Improved colors during world generation.
- Significantly reduced the use of warp during world generation.
- Parallelized and otherwise sped up significant parts of world generation.
- Various performance improvements to world generation.
- Nametags now a fixed size and shown in a limited range
- Non-humanoid skeletons now utilize configs for hotloading, and skeletal attributes.
- Names of NPCs spawned in the wild now include their species.

### Removed

- Remove heaptrack as it is now deprecated

## [0.4.0] - 2019-10-10

### Added

- Added adjustable FOV slider
- Added /explosion command
- Added first person switch
- Added single player server settings
- Added admin check for commands
- Started asset reloading system
- Added SRGB conversion in shaders
- Added adminify to give temp admin privilages

### Changed

- Collision and fall damage fixes
- Switched to eventbus system
- Improved seed generation, diffusion function
- Switch to hashbrown in server/client
- Improved colors and lighting
- Replaced view distance culling with frustum culling

## [0.3.0] - 2019-08-04

### Added

- Added enemies
- Added player info to debug window
- Added server info
- Game settings persist after closing
- Added caves
- Added random NPC names
- Added tree roots, houses, basic lights
- Added XP and leveling
- Added build mode
- Character customization, multiple races
- Inventories (WIP)
- Day/night, better shaders, voxel shadows

### Changed

- Fixed attack delay
- Fixed disclaimer to show only once
- Only send physics updates for entities within view distance
- Fix for headphones and invalid device parameters
- Fixed asset names for consistancy
- Fixes animals jumping after their target no matter how far\
- Improved SFX in caves
- Better combat, movement, and animations
- Many performance optimizations
- Better world generation, more biomes

## [0.2.0] - 2019-05-28

### Added

- Hang Gliding
- Pets: Pig and Wolf. They can be spawned with /pig and /wolf commands.
- Name tags: You can finally know who is this guy with the blue shirt!
- single player: No need to start a server just to play alone
- Character customization: It isn't fully complete but still allows you to look different than others
- Music!
- Major performance improvements related to the fact that we rewrote the entire game
- 0% chance to get a deadlock
- Animations: You finally can move your limbs!
- Combat: You can finally swing your sword that has been on your back. Enemies are coming soon, but you can always fight with other players
- When a server dies the game no longer crashes - you will be just kicked to the main menu

## [0.1.0] - 2018-XX-XX

_0.1.0 was part of the legacy engine_

[unreleased]: https://gitlab.com/veloren/veloren/compare?from=v0.8.0&to=master
[0.8.0]: https://gitlab.com/veloren/veloren/compare?from=v0.7.0&to=v0.8.0
[0.7.0]: https://gitlab.com/veloren/veloren/compare?from=v0.6.0&to=v0.7.0
[0.6.0]: https://gitlab.com/veloren/veloren/compare?from=v0.5.0&to=v0.6.0
[0.5.0]: https://gitlab.com/veloren/veloren/compare?from=v0.4.0&to=v0.5.0
[0.4.0]: https://gitlab.com/veloren/veloren/compare?from=v0.3.0&to=v0.4.0
[0.3.0]: https://gitlab.com/veloren/veloren/compare?from=v0.2.0&to=v0.3.0
[0.2.0]: https://gitlab.com/veloren/veloren/compare?from=7d17f8b67a2a6d5aa00730f028cedc430fd5075a&to=v0.2.0
[0.1.0]: https://gitlab.com/veloren/game
