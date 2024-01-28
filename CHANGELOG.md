# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Esperanto translation
- Item quantity sort in player inventory.
- Using Block('Alt' by default) in Defensive Stance now feels stronger
- Recipe for twigs from wooden logs
- First version of multisalvage that allows to obtain more than one piece of material from salvage
- Axe
- Combat music toggle
- Spawn rtsim wyverns that travel the world, providing dragon scale loot drops
- Hardwood in tropical forests, frostwood in cold forests, and iron wood on the top of giant trees
- Recipe for shovel, which is used to dig in mud and graves
- Recipe for a new leather pack
- Keybinds for zooming the camera (Defaults: ']' for zooming in and '[' for zooming out)
- Added the ability to make pets sit, they wont follow nor defend you in this state
- Portals that spawn in place of the last staircase at old style dungeons to prevent stair cheesing
- Mutliple singleplayer worlds and map generation UI.
- New arena building in desert cities, suitable for PVP, also NPCs like to watch the fights too
- The loading screen now displays status updates for singleplayer server and client initialization progress
- New Frost Gigas attacks & AI
- Allow plugins to add weapon and armor items
- New voxelised LoD shader effect
- Allow plugins to add recipes and item images
- `SnowGlitter` experimental shader.
- Crafting recipe for Cloverleaf glider.
- Burning Potion that applies the Burning effect to the user
- Precision
- A few new commands, `/tether`, `/destroy_tethers`, `/mount` and `/dismount`.
- A way to target non-player entities with commands. With rtsim_id: `rtsim@<id>`, with uid: `uid@<id>`.
- Shorthand in voxygen for specific entities in commands, some examples `@target`, `@mount`, `@viewpoint`.
- Added hit_timing to BasicMelee abilities
- A tavern building where npcs go to relax.
- Toggle for walking instead of running (Default: `I`).
- Added day duration slider configuration on map creation UI.
- Potion of Agility
- A way for servers to specify must-accept rules for players
- A flag argument type for commands
- The ability to turn lamp-like sprites on and off

### Changed

- Plugins now target wasm32-unknown-wasi and all wasm cfgs are gone
- Slightly reduced quantities of ingredients needed to craft cooked foods
- Improved and cleaned loot tables for T1 and T2 dungeons as well as large cave monsters (Good bye, Bowls and Stones!)
- Defensive Fell Strike's dmg raised
- Defensive Cascade's more effective against parried foes
- Defensive Riposte's buildup duration raised a bit
- Capabilities of strikes to parry & block now more reliable
- Defensive Disengage now more responsive and can block melee
- Deflect no longer parry melee hits
- Changed recipes for some bags to make them more horizontal
- Increase invetory slots on some bags to improve early game experience
- Made helmets, necklaces, rings, twig armors and some gliders salvageable
- Tweaked stats on some foods so they generally increase a tiny bit more HP
- Reduced idle time after consumption from 5 to 4 seconds
- Reduced interaction time for harvestable and collectible items to smooth the gameplay
- Gliders no longer drop from cave creatures
- Tweaked Archaeos, Basilisk, Dreadhorn, Dullahan, Mammoth, Ngoubou, Ntouka and Roshwalr loot tables to be a bit more rewarding
- Removed weapon and armor drops from standard NPCs
- Tweaked dungeons mobs and chests loot tables to be more balanced and rewarding
- Changed iron ore to iron ingots in the instruments' recipes
- Changed gold ore to gold ingots in the Brinstone armor set recipes
- Updated windowing library, wayland may work better.
- Portal model has been updated by @Nectical
- Chat command responses sent by the server can now be localized
- Frost Gigas spawns in cold areas (but isn't forced to stay there)
- The ability limit for non-humanoids has been removed
- Improved running, wielding, and riding animations
- Fixed offset of items carried on backs when wearing cloaks and backpacks
- Linearize light colors on the CPU rather than in shaders on the GPU
- You can no longer stack self buffs
- Renamed "Burning Potion" to "Potion of Combustion"
- Render LoD terrain on the character selection screen
- Camera no longer jumps on first mouse event after cursor grab is released on macos
- Updated wgpu. Now supports OpenGL. Dx11 no longer supported.

### Removed
- Medium and large potions from all loot tables
- LoD pop-in effect
- Removed Dullahans from halloween event
- Random critical hits

### Fixed
- Fixed wild roaming cyclop loot table to not drop the quarry key
- Dungeons now have an outer wall, preventing them from intersecting with caves or leaving holes in sides of mountains.
- Location names are displayed in character selection dialog
- You can no longer write messages to old groups after being kicked and not having updated your chat mode.
- Location names are now also correct after editing and creating characters
- NPC's wont pick up recently dropped items (if not hostile towards you)
- Fixed "low fps" of different shaders caused by low floating point precision when using time.
- Fixed bug where airship captains would mass generate after using /reload_chunks
- Fixed french translation "Énergie Consommée" -> "Regain d'Énergie"
- Fixed Perforate icon not displaying
- Make cave entrances easier to follow
- Renamed Twiggy Shoulders to match the Twig Armor set
- No longer stack buffs of the same kind with equal attributes, this could lead to a DoS if ie. an entity stayed long enough in lava.
- Nerfed Earthsplitter

## [0.15.0] - 2023-07-01

### Added

- Command to toggle experimental shaders.
- Faster Energy Regeneration while sitting.
- Lantern glow for dropped lanterns.
- Suggests commands when an invalid one is entered in chat and added Client-side commands to /help.
- Moderator badge in the chat.
- More aggressive scene culling based on camera position to improve performance.
- Some chests requiring lockpicks or keys.
- Unlockable door blocks.
- Sprite rotation for Spots.
- Better entity placement options for spots.
- Camera zoom can now be locked, to prevent accidental zooming while rolling in combat. It comes
  with a keybind to enable/disable the setting, and an Auto/Toggle behavior setting. Auto behavior
  will only lock the camera zoom while movement and combat inputs are also being pressed.
- Custom spots can be added without recompilation (only ron and vox files)
- Setting in userdata/server/server_config/settings.ron that controls the length of each day/night cycle.
- Starting site can now be chosen during character creation
- Durability loss of equipped items on death
- Reputation system: crimes will be remembered and NPCs will tell each other about crimes they witness
- NPCs will now talk to players and to each other
- NPCs now have dedicated professions and will act accordingly
- NPCs other than merchants can be traded with
- NPCs will seek out a place to sleep when night comes
- Merchants now travel between towns
- Travellers and merchants will stay a while in each town they visit and converse with the locals
- Resource tracking: resources in the world can be temporarily exhausted, requiring time to replenish
- Airships now have pilot NPCs
- Simulated NPCs now have repopulation mechanics
- NPCs now have unique names
- A /scale command that can be used to change the in-game scale of players
- Merchants will flog their wares in towns, encouraging nearby character to buy goods from them
- NPCs will now tell you about nearby towns and how to visit them
- NPCs will migrate to new towns if they are dissatisfied with their current town
- Female humanoids now have a greeting sound effect
- Loot that drops multiple items is now distributed fairly between damage contributors.
- Added accessibility settings tab.
- Setting to enable subtitles describing sfx.
- Item drops that are spatially close and compatible will now merge with one-another to reduce performance problems.
- Airships can now have sprites, which can be interacted with.
- Some sprites can be sat on.
- Pet birds can now sit on the player's shoulder as they explore the world.
- Adlet caves
- Durability free areas (`/area_add <area_name> no_durability ...`)
- Added Brazilian Portuguese translation.
- Added additional confirmation when trading for nothing.
- Esperanto translation
- Item quantity sort in player inventory.

### Changed

- Bats move slower and use a simple proportional controller to maintain altitude
- Bats now have less health
- Climbing no longer requires having 10 energy
- Castles will now be placed close to towns
- Sword
- Rescaling of images for the UI is now done when sampling from them on the GPU. Improvements are
  particularily noticeable when opening the map screen (which involves rescaling a few large
  images) and also when using the voxel minimap view (where a medium size image is updated often).
- Towns now have a variety of sizes
- The game now starts in fullscreen by default
- Default audio volume should be less likely to destroy ear drums
- Creatures flee less quickly when low on health
- All `/build_area_*` commands have been renamed to `/area_*`, and you will have to pass an additional area type
- Collision damage can now be applied in horizontal axes, in addition to the vertical axis
- Items will vanish after 5 minutes to minimise performance problems
- The language identifiers used by the i18n translation system have been converted to IETF BCP 47 (RFC 5646) language tags.
- Improved particle performance for lava and leaves
- The wander-radius of entities can be defined in their .ron config now
- Dwarven-Mine themed dungeon
- Multiple item types can be dropped from enemies and chests now
- Readable signs
- Plugins now target wasm32-unknown-wasi and all wasm cfgs are gone
- Slightly reduced quantities of ingredients needed to craft cooked foods
- Improved and cleaned loot tables for T1 and T2 dungeons as well as large cave monsters (Good bye, Bowls and Stones!)
- Added coastal towns

### Removed

- Plugins can no longer prevent users from logging in

### Fixed

- Doors
- Debug hitboxes now scale with the `Scale` component
- Potion quaffing no longer makes characters practically immortal.
- Stat diff now displays correctly for armor
- Lamps, embers and campfires use glowing indices
- Non-potion drinks no longer heal as much as potions.
- Added SFX to the new sword abilities
- Fixed various issues with showing the correct text hint for interactable blocks.
- Intert entities like arrows no longer obstruct interacting with nearby entities/blocks.
- Underwater fall damage
- The scale component now behaves properly
- Multiple model support for dropped items (orichalcum armor)
- Made rtsim monsters not go into too deep water, and certainly not outside the map.
- Fixed bug where npcs would be dismounted from vehicles if loaded/unloaded in a certain order.
- Fixed a slow leak on the server where Uid -> Entity mappings weren't cleaned up.
- Clients going back into the character screen now properly have their old entity cleaned up on
  other clients.

## [0.14.0] - 2023-01-07

### Added

- Setting for disabling flashing lights
- Spectate mode for moderators.
- Currently playing music track and artist now shows in the debug menu.
- Added a setting to influence the gap between music track plays.
- Added a Craft All button.
- Server: Vacuum database on startup
- SeaChapel, greek/latin inspired dungeon for ocean biome coasts
- Entity view distance setting added (shown in graphics and network tabs). This setting controls
  the distance at which entities are synced to the client and which entities are displayed in.
  This is clamped to be no more than the current overall view distance setting.
- View distance settings that are lowered by the server limit (or other factors) now display an
  extra ghost slider cursor when set above the limit (instead of snapping back to the limit).
  Limits on the view distance by the server no longer affect the settings saved on the client.
- HQX upscaling shader for people playing on low internal resolutions
- Pets can now be traded with.
- Crafting recipe for black lantern
- Added redwood and dead trees
- Water will now move according to its apparent flow direction
- Added screen-space reflection and refraction shaders
- Added reflection quality setting
- UI: Added a poise indicator to the player's status bars
- FxUpscale AA mode for higher quality graphics at reduced internal resolutions
- Graphics presets
- Sword
- Doors now animate opening when entities are near them.
- Musical instruments can now be crafted, looted and played
- NPCs now move to their target's last known position.
- Experience bar below the hotbar
- Bridges.
- Tool for exporting PNG images of all in-game models (`cargo img-export`)
- Calendar event soundtracks.

### Changed

- Use fluent for translations
- First tab on Login screen triggers username focus
- Certain NPCs will now attack when alone with victim
- /kill_npcs no longer leaves drops behind and also has bug causing it to not destroy entities
  fixed.
- Default present mode changed to Fifo (aka 'Vsync capped').
- Old "Entity View Distance" setting renamed to "Entity Detail Distance" (since this controls the
  distance at which lower detail models are used for entities).
- Present mode options renamed for clarity: Fifo -> 'Vsync capped', Mailbox -> 'Vsync uncapped',
  Immediate -> 'Vsync off'.
- Item pickup UI now displays items that members of your group pick up.
- Improved shiny water shaders
- Tweaked armor stats
- Move bag icon to skillbar
- Improved inventory sorting by Category

### Removed

### Fixed

- Fixed npc not handling interactions while fighting (especially merchants in trade)
- Fixed bug where you would still be burning after dying in lava.
- Workaround for rayon bug that caused lag spikes in slowjobs
- Fixed crash due to zooming out very far
- Client properly knows trade was cancelled when exiting to the character screen (and no longer
  tries to display the trade window when rejoining)
- Cancel trades for an entity when it is deleted (note this doesn't effect trades between players
  since their entities are not removed).
- Fixed bug where the view distance selection was not immediately applied to entity syncing when
  first joining a server and when changing the view distance (previously this required moving to a
  new chunk for the initial setting or subsequent change to apply).
- Moderators and admins are no longer blocked from logging in when there are too many players.
- FXAA now behaves correctly at non-1.0x internal resolutions
- Pets no longer aggro on pet owners after being healed
- Pets no longer lose their intrinsic weapons/armour when loaded on login.
- Fixed npcs using `/say` instead of `/tell`
- Camera jittering in third person has been significantly reduced
- Many water shader issues have been fixed
- Flee if attacked even if attacker is not close.
- `/time` command will never rewind time, only advance it to not break rtsim

## [0.13.0] - 2022-07-23

### Added

- Chat commands to mute and unmute players
- Waypoints saved between sessions and shared with group members.
- New rocks
- Weapon trails
- Hostile agent will now abort pursuing their target based on multiple metrics
- Admin command to reload all chunks on the server
- Furniture and waypoints in site2 towns
- Text input for trading
- Themed Site CliffTown, hoodoo/arabic inspired stone structures inhabited by mountaineer NPCs.
- NPCs now have rudimentary personalities
- Added Belarusian translation
- Add FOV check for agents scanning for targets they are hostile to
- Implemented an LoD system for objects, making trees visible far beyond the view distance
- Add stealth stat on Bag UI
- Water caves
- Modular weapons
- Added Thai translation
- Skiing and ice skating
- Added loot ownership for NPC drops
- Bamboo collectibles now spawn near rivers
- Chest sprites can longer be exploded
- Smoke varies by temperature, humidity, time of day and house
- Added loot ownership for drops from mining
- Added an option for experience number accumulation.
- Added an option for damage number rounding (when greater than or equal to 1.0).
- Added sliders for incoming/non-incoming damage accumulation duration.
- New ambience sounds
- Slider for ambience volume
- Weather generated on server is sent to clients, and seen on clients as rain/clouds.
- Updated Brazilian Portuguese Translation
- Lightning storms
- More varied ambient birdcalls
- Cave biomes
- Updated the Polish translation

### Changed

- Improved site placement
- [Server] Kick clients who send messages on the wrong stream
- Reworked Merchant trade price calculation, Merchants offer more wares
- Enable new giant trees, changed what entities spawn at them
- Stealth is now shown as a percentage in Stats Diary UI
- Stealth effects from sneaking and armor are evaluated independently. Armor now has effects even when not sneaking
- Zoom-in effect when aiming bow is now optional
- Non-Humanoid NPCs now pick up consumables when less than full health and use them to heal up.
- Changed module component modifier costs to the following scheme, based on base material: 1 -> 2 -> 5 -> 10 -> 15 -> 25
- Damage from the same source dealt in the same tick will now be grouped up.
- Critical hits are now shown differently in the damage numbers.
- Fall damage and some (extra) buffs/debuffs now show up in the damage numbers.
- Optimized sprite processing decreasing the startup time of voxygen (and long freezes when trying
  to enter the world when this hasn't finished).
- Metadata added to music files. Listen to the soundtrack more easily!
- Overhauled caves: they're now a multi-layer network spanning the entire world

### Removed

- Removed the options for single and cumulated damage.

### Fixed

- Fixed bug that would sometimes cause taking a screenshot to panic because a buffer was mapped at the wrong time.
- Players can no longer push waypoints around
- Sites will now also be placed near the edge of the map
- Fix a bug causing NPCs to jitter on interaction and randomly run away.
- Harvester boss arenas should be more accessible and easier to exit
- Fix agents not idling
- Fixed an error where '{amount} Exp' floater did not use existing localizations
- Fix villagers seeing cultists and familiar enemies through objects.
- Menacing agents are now less spammy with their menacing messages
- Fixed the title screen FPS cap not applying when the background FPS limit was set higher than 60 FPS
- Fixed an issue where the hurt animation would "jump" whenever you lost/gained health.
- Fixed a bug where multiple damage sources in the same tick would show up as a singular attack.
- Fixed an issue where, if the same amount of healing and damage was received in the same tick, nothing would be shown.
- UI sfx now play from UI instead of from camera (allowing stereo sfx)
- Most sfx now correctly play when camera is underwater
- All sounds now stop upon quitting to main menu
- Combat music now loops and ends properly
- Modular weapons now have a selling price
- Closing a subwindow now only regrabs the cursor if no other subwindow requires it.

## [0.12.0] - 2022-02-19

### Added

- Added a setting to always show health and energy bars
- Added a crafting station icon to the crafting menu sidebar for items that could be crafted at a crafting station
- Added a setting to disable the hotkey hints
- Added a credits screen in the main menu which shows attributions for assets
- Shrubs, a system for spawning smaller tree-like plants into the world.
- Waterfalls
- Sailing boat (currently requires spawning in)
- Added a filter search function for crafting menu, use "input:______" to search for recipe inputs
- Added catalan (Catalonia) language translation
- Sneaking with weapons drawn
- Stealth stat values on (some) armors
- All new dismantling interface found at your nearest dismantling staion
- Wearable headgear, including hood, crown, bandanas
- Bomb sprites (can be exploded with arrows or other explosions)
- Campfire waypoints in towns
- Arbitrary volume entities
- New outfit for merchants
- Nightly linux Aarch64 builds are now produced (distribution via airshipper will follow soon)
- Worldgen wildlife density modifier in features.ron
- Rivers now make ambient sounds (again)
- Added a setting to see own speech bubbles
- Added an option to allow players to remove keybindings
- Piercing damage now ignores an amount of protection equal to damage value
- Slashing damage now reduces target's energy by an amount equal to damage dealt to target post-mitigation
- Crushing damage now does poise damage to a target equal to the amount mitigated by armor
- UI to select abilities and assign to hotbar
- Position of abilities on hotbar is now persisted through the server
- Interation hints now appear for sprites and entities
- Players can now mount and ride pets
- Experimental shaders, that can be enabled in Voxygen's settings (see the book for more information)
- Keybinding customization to set waypoint on Map
- Added arthropods
- A 'point light glow' effect, making lanterns and other point lights more visually pronounced
- Generate random name for site2 sites
- Shader dithering to remove banding from scenes with large colour gradients
- Convert giant trees to site2
- Add new upgraded travelers
- Wallrunning

### Changed

- Made dungeon tiers 3, 4, and 5 more common
- Put date at the begining of the log file instead of the end to allow MIME type recognition
- Tweaked CR and exp calculation formula
- Sprite spawn rates
- The Interact button can be used on campfires to sit
- Made map icons fade out when near the edge of the map display
- Roughly doubled the speed of entity vs terrain physics checks
- Updated client facing error messages to be localizable strings
- Nerfed some skill values
- Tweaked critical chance of legendary weapons
- Agents using fireball projectiles aim at the feet instead of the eyes
- Explosions can now have a nonzero minimum falloff
- EXP on kill is now shared based on damage contribution
- Dungeons have somewhat proper scaling. The higher the dungeon the harder it gets, Cultist staying unchanged while Mino is now at its level.
- Parallelized entity sync system on the server
- Item color backgrounds are now lighter
- All items that used the PNG file format now have a VOX file instead
- Yeti loot table modified
- Phoenix feathers are now Legendary quality
- Green/Red lantern now shine their respective color instead of the default lantern color
- Poise damage dealt to a target that is in a stunned state is now converted to health damage at an efficiency dependent on the severity of the stunned state
- You are now immune to poise damage for 1 second after leaving a stunned state
- Removed or reduced poise damage from most abilities
- Made the hotbar link to items by item definition id and component composition instead of specific inventory slots.
- Made loot boxes drop items instead of doing nothing in order to loot forcing
- Refactored agent code file structure
- Changed the way light strength is rendered by moving processing from shader code (GPU) to CPU code
- Bumped tracing-subscriber to resolve [RUSTSEC-2022-0006](https://rustsec.org/advisories/RUSTSEC-2022-0006)
- Made /home command a mod+ exclusive
- Friendly creatures will now defend each other
- Creatures will now defend their pets
- [WorldGen] Change path colors
- Render item drops instead of placeholder textures
- Arthropods are rebalanced
- Slight hat item rebalance (hats are more specialized and befitting of their rarity rank)
- Harvester boss buffed in stats

### Removed

- Removed unused PNG files
- Removed bomb_pile

### Fixed

- The menu map now properly handles dragging the map, zooming, and setting the waypoint when hovering icons
- Falling through an airship in flight should no longer be possible (although many issues with airship physics remain)
- Avoided black hexagons when bloom is enabled by suppressing NaN/Inf pixels during the first bloom blur pass
- Many know water generation problems
- Trading over long distances using ghost characters or client-side exploits is no longer possible
- Merchant cost percentages displayed as floored, whole numbers
- Bodies of water no longer contain black chunks on the voxel minimap.
- Agents can flee once again, and more appropriately
- Items in hotbar no longer change when sorting inventory
- Lantern color changes when swapping lanterns
- NPCs no longer wander off cliffs
- Guards will defend villagers instead of simply threatening the attacker
- Seafaring ships no longer spawn on dry land

## [0.11.0] - 2021-09-11

### Added

- Added a skill tree for mining, which gains xp from mining ores and gems.
- Added debug line info to release builds, enhancing the usefulness of panic backtraces
- NPCs and animals can now make sounds in response to certain events
- Players can press H to greet others
- Ability to toggle chat visibility
- Added gem rings with various stat improvements.
- Animations for using consumables.
- New danari character customizations
- Bald hairstyles for humans and danari
- AI for sceptre wielders and sceptre cultists in Tier 5 dungeons
- HUD debug info now displays current biome and site
- Quotes and escape codes can be used in command arguments
- Toggle chat with a shortcut (default is F5)
- Pets are now saved on logout 🐕 🦎 🐼
- Dualwielded, one-handed swords as starting weapons (Will be replaced by daggers in the future!)
- Healing sceptre crafting recipe
- NPCs can now warn players before engaging in combat
- Custom error message when a supported graphics backend can not be found
- Add server setting with PvE/PvP switch
- Can now tilt glider while only wielding it
- Experimental terrain persistence (see server documentation)
- Add GPU filtering using WGPU_ADAPTER environment variable
- Explosions no longer change block colours within safe zones
- The 'spot' system, which generates smaller site-like structures and scenarios
- Chestnut and cedar tree varieties
- Shooting sprites, such as apples and hives, can knock them out of trees
- Sprite pickup animations
- Add VELOREN_ASSETS_OVERRIDE variable for specifying folder to partially override assets.
- Cultist raiders
- Bloom Slider

### Changed

- Entity-entity pushback is no longer applied in forced movement states like rolling and leaping.
- Updated audio library (rodio 0.13 -> 0.14).
- Improve entity-terrain physics performance by reducing the number of voxel lookups.
- Clay Golem uses shockwave only after specific fraction of health and other difficulty adjustments.
- Made strafing slightly slower
- Food now has limited regeneration strength but longer duration.
- Harvester boss now has new abilities and AI
- Death particles and SFX
- Default keybindings were made more consistent
- Adjusted Yeti difficulty
- Now most of the food gives Saturation in the process of eating
- Mushroom Curry gives long-lasting Regeneration buff
- Trades now consider if items can stack in full inventories.
- The types of animals that can be tamed as pets are now limited to certain species, pending further balancing of pets
- Made server-cli admin add/remove command use positional arguments again
- Usage of "stamina" replaced with "energy"
- Glider dimensions now depend on character height
- Glider dimensions somewhat increased overall
- Dungeon difficulty level starts at 1 instead of 0
- The radius of the safe zone around the starting town has been doubled
- NPCs can sometimes drop no loot at all

### Removed

- Enemies no longer spawn in dungeon boss room
- Melee critical hit no longer applies after reduction by armour
- Enemies no more spawn in dungeon boss room
- Melee critical hit no more applies after reduction by armour
- Removed Healing Sceptre as a starting weapon as it is considered an advanced weapon
- The ability to pickup sprites through walls

### Fixed

- Crafting Stations aren't exploadable anymore
- Cases where no audio output could be produced before.
- Significantly improved the performance of playing sound effects
- Dismantle and Material crafting tabs don't have duplicated recipes
- Campfires now despawn when underwater
- Players no longer spawn underground if their waypoint is underground
- Map will now zoom around the cursor's position and drag correctly
- No more jittering while running down slopes with the glider out
- Axe normal attack rewards energy without skill points
- Gliders no longer suffer from unreasonable amounts of induced drag
- Camera is now clipping a lot less

## [0.10.0] - 2021-06-12

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
- Quality color indicators next to recipe names in crafting menu
- New cave visuals: Ridges, pits, new sprites, colors
- Veins in caves to dig through to uncover ore
- Armor material system with 6 armor sets each in hide, mail and cloth categories
- New armor stats including max energy, energy reward, critical hit damage
- Meat drops from animals
- New ores, plants and hides to be looted from the world and processed into craft ingredients
- Added more crafting stations, loom, spinning wheel, tanning rack, forge

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
- Reworked yeti to have unique attacks
- Widened recipe name list in crafting menu
- Reworked animal loot tables
- NPC hitboxes better fit their model.

### Removed

- Removed command: "debug", use "/kit debug" instead
- Gravity component has been removed
- In-air movement has been removed
- Energy cost of deploying the glider has been removed
- Removed steel and cultist loot tables

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
- The login and character selection screens no longer cause high GPU usage when the framerate limit is set to Unlimited.
- Deadwood will now attack targets who are at different elevations than itself.

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
- Added authentication system (to play on the official server register on <https://account.veloren.net>)
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
- Remove coin counter at the bottom of inventories.

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
- Fixes animals jumping after their target no matter how far
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

[unreleased]: https://gitlab.com/veloren/veloren/compare?from=v0.15.0&to=master
[0.15.0]: https://gitlab.com/veloren/veloren/compare?from=v0.14.0&to=v0.15.0
[0.14.0]: https://gitlab.com/veloren/veloren/compare?from=v0.13.0&to=v0.14.0
[0.13.0]: https://gitlab.com/veloren/veloren/compare?from=v0.12.0&to=v0.13.0
[0.12.0]: https://gitlab.com/veloren/veloren/compare?from=v0.11.0&to=v0.12.0
[0.11.0]: https://gitlab.com/veloren/veloren/compare?from=v0.10.0&to=v0.11.0
[0.10.0]: https://gitlab.com/veloren/veloren/compare?from=v0.9.0&to=v0.10.0
[0.9.0]: https://gitlab.com/veloren/veloren/compare?from=v0.8.0&to=v0.9.0
[0.8.0]: https://gitlab.com/veloren/veloren/compare?from=v0.7.0&to=v0.8.0
[0.7.0]: https://gitlab.com/veloren/veloren/compare?from=v0.6.0&to=v0.7.0
[0.6.0]: https://gitlab.com/veloren/veloren/compare?from=v0.5.0&to=v0.6.0
[0.5.0]: https://gitlab.com/veloren/veloren/compare?from=v0.4.0&to=v0.5.0
[0.4.0]: https://gitlab.com/veloren/veloren/compare?from=v0.3.0&to=v0.4.0
[0.3.0]: https://gitlab.com/veloren/veloren/compare?from=v0.2.0&to=v0.3.0
[0.2.0]: https://gitlab.com/veloren/veloren/compare?from=7d17f8b67a2a6d5aa00730f028cedc430fd5075a&to=v0.2.0
[0.1.0]: https://gitlab.com/veloren/game
