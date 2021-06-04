# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- New Skills for Climbing: Climbing Speed and Climbing Cost
- Pickaxes (can be used to collect gems and mine weak rock)
- You can now jump out of rolls for a slight jump boost
- Dungeons now have multiple kinds of stairs.
- Trades now display item prices in tooltips.
- Admin designated build areas
- Indicator text to collectable terrain sprites
- You can now autorequest exact change by ctrl-clicking in a trade, and can quick-add individual items with shift-click.
- Buy and sell prices in tooltips when trading with a merchant now have colors.
- Attacks now emit sound effects from the target on hit.
- Crafting menu tabs
- Auto camera setting, making the game easier to play with one hand
- Topographic map option
- Search bars for social and crafting window
- RTsim travellers now follow paths between towns
- "Poise" renamed to "Stun resilience"
- Stun resilience stat display
- Villagers and guards now spawn with potions, and know how to use them.
- Combat music in dungeons when within range of enemies.
- New Command: "kit", place a set of items into your inventory
- Added --sql-log-mode profile/trace parameter to veloren-server-cli
- Added /disconnect_all_players admin command
- Added disconnectall CLI command
- One handed weapons can now be used and found in the world
- Players can now opt-in to server-authoritiative physics in gameplay settings.
- Added `/server_physics` admin command.
- Sort inventory button
- Option to change the master volume when window is unfocused
- Crafting stations in towns
- Option to change the master volume when window is unfocused
- Entities now have mass
- Entities now have density
- Buoyancy is calculated from the difference in density between an entity and surrounding fluid
- Drag is now calculated based on physical properties
- Terrain chunks are now deflate-compressed when sent over the network.
- Missing translations can be displayed in English.
- New large birds npcs
- Day period dependant wildlife spawns
- You can now block and parry with melee weapons
- Lift is now calculated for gliders based on dimensions (currently same for all)
- Specific music tracks can now play exclusively in towns.
- Custom map markers can be placed now
- Fundamentals/prototype for wiring system
- Mountain peak and lake markers on the map
- There's now a checkbox in the graphics tab to opt-in to receiving lossily-compressed terrain colors.
- /buff command which allows you to cast a buff on player
- Warn the user with an animated red text in the second phase of a trade in which a party is offering nothing.
- /skill_preset command which allows you to apply skill presets
- Added timed bans and ban history.
- Added non-admin moderators with limit privileges and updated the security model to reflect this.
- Added a minimap mode that visualizes terrain within a chunk.
- Chat tabs
- NPC's now hear certain sounds
- Renamed Animal Trainers to Beastmasters and gave them their own set of armor to wear
- ChargedRanged attacks (such as some bow attacks) use an FOV zoom effect to indicate charge.
- Add chest to each dungeon with unique loot
- Added a new option in the graphics menu to enable GPU timing (not always supported). The timing values can be viewed in the HUD debug info (F3) and will be saved as chrome trace files in the working directory when taking a screenshot.
- Added new Present Mode option in the graphics menu. Selecting Fifo (i.e. vsync) or Mailbox can be used to eliminate screen tearing.

### Changed

- Admins can now grant normal players plots to place blocks within
- Diamonds are now much more than twice as expensive as twigs.
- Permission to build is no longer tied to being an admin
- Separated character randomization buttons into appearance and name.
- Reworked mindflayer to have unique attacks
- Glowing remains are now `Armor` instead of `Ingredients`.
- Generated a new world map
- Overhauled clouds for more verticality and performance
- New tooltip for items with stats comparison
- Improved bow feedback, added arrow particles
- Retiered most sceptres and staves
- Loot tables can now recursively reference loot tables
- "max_sfx_channels" default now set to 30
- Merchants now have stacks of stackable items instead of just one per slot
- Bag tooltips only show slots now
- Removed infinite armour values from most admin items
- Item tooltips during trades will now inform the user of what ctrl-click and shift-click do
- International keyboards can now display more key names on Linux and Windows instead of `Unknown`.
- There is now a brief period after a character leaves the world where they cannot rejoin until their data is saved
- Certain uses of client-authoritative physics now subject the player to server-authoritative physics.
- Dodge roll iframes and staff explosion are now unlocked by default, with points refunded for existing characters.
- Dash melee now stops after hitting something. Infinite dash also now replaced with dash through.
- Collisions, knockbacks, jumping and drag are now physical forces applied to the entity's body mass
- Turning rate has been made more consistent across angles
- Gravity has been lowered so that physics can work more reasonably
- Jump has been decreased in height but extended in length as a result of the new gravity
- Fall damage has been adjusted with the new gravity in mind
- Projectiles now generally have a different arc because they no longer have their own gravity modifier
- Increased agent system target search efficiency speeding up the server
- Added more parallelization to terrain serialization and removed extra cloning speeding up the server
- Energy now recharges while gliding
- Debug Kit is split to "admin_cosmetics" and "debug"
- Potion Kit is renamed to "consumables" and gives potions and mushroom curry
- Cultist Kit gives cape, rings and necklace in addition to armour and weapons.
- Reworked minotaur to have unique attacks.
- Wiring is now turing complete
- Better active/inactive master sound slider logic
- Cultist Husk no longer drops weapons and armor
- Animal Trainers now spawn in tier-5 dungeon and not in tier-3
- Reworked clay golem to have unique attacks.
- Merchants now use `/tell` instead of `/say` to communicate prices
- Entities catch on fire if they stand too close to campfires
- Water extinguishes entities on fire
- Item pickups are shown in separate window and inventory-full shows above item
- Reworked bow
- Switched to the `wgpu` graphics library giving us support for vulkan, dx12, metal, and dx11 (support for opengl is lost for the moment). This improves the graphics performance for many users.
- Reworked sprite rendering to vastly reduce the CPU work. Large sprite view distances are now much more performant.
- Optimized rendering of quads (most of the graphics in the game) using an index buffer, decreasing the number of vertices that need to be processed by 33%.
- Moved the rest of screenshot work into the background. Screenshoting no longer induces large pauses.
- Reworked tidal warrior to have unique attacks

### Removed

- Removed command: "debug", use "/kit debug" instead
- Gravity component has been removed
- In-air movement has been removed
- Energy cost of deploying the glider has been removed

### Fixed

- Server kicks old client when a user is trying to log in again (often the case when a user's original connection gets dropped)
- Added a raycast check to beams to prevent their effect applying through walls
- Flying agents raycast more angles to check for obstacles.
- Mouse Cursor now locks to the center of the screen when menu is not open
- Social window no longer moves when group is open
- Combat rating no longer takes buffs into account
- Minimap icons are now displayed in both map modes
- Server now denies any running trades when a user exits to the character selection screen.
- Sfx volume changes now also change the ambient sounds volume
- Staff fire shockwave ability no longer has an unlimited vertical range
- Skillbar buttons correctly account for skill points when checking if player has enough stamina for the ability.
- Burning Debuff icon is now displayed correctly.
- Villagers in safezones no longer spam messages upon seeing an enemy
- Wolf AI will no longer circle into walls and will instead use the power of raycasts to stop early
- Squirrels are no longer immune to arrows at some angles.
- /spawn command's auto-complete now works for species names
- Mindflayer AI now correctly summons husks at certain HP thresholds.
- Far away NPCs respond to being damaged by a projectile
- Fixed terrain clipping with glider
- Fixed an issue where prices weren't properly making their way from econsim to the actual trade values.
- Fixed entities with voxel colliders being off by one physics tick for collision.
- Airships no longer oscillate dramatically into the sky due to mistaking velocity for acceleration.

## [0.9.0] - 2021-03-20

### Added

- Plugin can now retrieve data from ECS
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
- Poise system
- Snow particles
- Basic NPC interaction
- Lights in dungeons
- Trading system (bound to the `R` key by default, currently only works with players)
- Support for dual wielding (not accessible as animations still needed)
- Support for modular weapons.
- Saturation buff (healing from food) now queues
- Coral reefs, kelp forests, and seagrass
- Talk animation
- New bosses in 5 lower dungeons
- New enemies in 5 lower dungeons
- Added on join event in plugins
- Item stacking and splitting
- Procedural trees (currently only oaks and pines are procedural)
- Cliffs on steep slopes
- Giant tree sites
- Reset button for graphics settings
- Gave weapons critical strike {chance, multiplier} stats
- A system to add glow and reflection effects to figures (i.e: characters, armour, weapons, etc.)
- Merchants will trade wares with players
- Airships that can be mounted and flown, and also walked on (`/airship` admin command)
- RtSim airships that fly between towns.

### Changed

- Doubled range of ScaleMode slider when set to Custom
- Glider can now be deployed mid-air at the cost of some stamina based on fall speed
- Translations are now folders with multiple files instead of a huge single file
- Default inventory slots reduced to 18 - existing characters given 3x 6-slot bags as compensation
- Protection rating was moved to the top left of the loadout view
- Changed camera smoothing to be off by default.
- Footstep SFX is now dependant on distance moved, not time since last play
- Adjusted most NPCs hitboxes to better fit their models.
- Changed crafting recipes involving shiny gems to use diamonds instead.
- Cave scatter now includes all 6 gems.
- Adjusted Stonework Defender loot table to remove mindflayer drops (bag, staff, glider).
- Made humanoid NPCs use gliders (if equipped) when falling
- Changed default controller key bindings
- Improved network efficiency by ≈ factor 10 by using tokio.
- Added item tooltips to trade window.
- "Quest" given to new players converted to being a short tutorial
- Items can be requested from the counterparty's inventory during trade.
- Savanna grasses restricted to savanna, cacti to desert.
- Fireworks recursively shoot more fireworks.
- Improved static light rendering and illumination
- Improved the tree spawning model to allow for overlapping forests
- Changed sunlight (and, in general, static light) propagation through blocks to allow for more material properties
- Overhauled the sceptre
- Make the /time command relative to the current day
- Spatial partitioning via a grid for entity versus entity collisions was added which can more than halve the total tick time at higher entity counts (> ~1000)
- Improved efficency of entity versus terrain collisions (they now take less than half the time)
- The loading screen will now display random animations

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
- Fixed AI behavior so only humanoids will attempt to roll
- Fixed missing GameInputs (sneak, swimup, swimdown) in controller mapping
- Fixed missing controller actions (dance and crafting)
- Fixed a bug where the stairs to the boss floor in dungeons would sometimes not spawn
- Fixed waypoints being placed underwater
- Objects and golems are not affected by bleed debuff anymore
- Fixed RtSim entity memory loss
- Mandated that merchants not wander away during a trade
- Fixed the villager conception of evil by encouraging them to react violently to characters wearing cultist gear

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

[unreleased]: https://gitlab.com/veloren/veloren/compare?from=v0.9.0&to=master
[0.9.0]: https://gitlab.com/veloren/veloren/compare?from=v0.8.0&to=v0.9.0
[0.8.0]: https://gitlab.com/veloren/veloren/compare?from=v0.7.0&to=v0.8.0
[0.7.0]: https://gitlab.com/veloren/veloren/compare?from=v0.6.0&to=v0.7.0
[0.6.0]: https://gitlab.com/veloren/veloren/compare?from=v0.5.0&to=v0.6.0
[0.5.0]: https://gitlab.com/veloren/veloren/compare?from=v0.4.0&to=v0.5.0
[0.4.0]: https://gitlab.com/veloren/veloren/compare?from=v0.3.0&to=v0.4.0
[0.3.0]: https://gitlab.com/veloren/veloren/compare?from=v0.2.0&to=v0.3.0
[0.2.0]: https://gitlab.com/veloren/veloren/compare?from=7d17f8b67a2a6d5aa00730f028cedc430fd5075a&to=v0.2.0
[0.1.0]: https://gitlab.com/veloren/game
