# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
- Rebuilt quadruped_medium animation and assets
- Disabled destruction of most blocks by explosions

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

[unreleased]: https://gitlab.com/veloren/veloren/compare?from=v0.6.0&to=master
[0.6.0]: https://gitlab.com/veloren/veloren/compare?from=v0.5.0&to=v0.6.0
[0.5.0]: https://gitlab.com/veloren/veloren/compare?from=v0.4.0&to=v0.5.0
[0.4.0]: https://gitlab.com/veloren/veloren/compare?from=v0.3.0&to=v0.4.0
[0.3.0]: https://gitlab.com/veloren/veloren/compare?from=v0.2.0&to=v0.3.0
[0.2.0]: https://gitlab.com/veloren/veloren/compare?from=7d17f8b67a2a6d5aa00730f028cedc430fd5075a&to=v0.2.0
[0.1.0]: https://gitlab.com/veloren/game
