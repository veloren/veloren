# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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


### Changed

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
- Fixed singleplayer crash
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

### Removed

- Remove heaptrack as it is now deprecated

## [0.4.0] - 2019-10-10

### Added

- Added adjustable FOV slider
- Added /explosion command
- Added first person switch
- Added singleplayer server settings
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
- Singleplayer: No need to start a server just to play alone
- Character customization: It isn't fully complete but still allows you to look different than others
- Music!
- Major performance improvements related to the fact that we rewrote the entire game
- 0% chance to get a deadlock
- Animations: You finally can move your limbs!
- Combat: You can finally swing your sword that has been on your back. Enemies are coming soon, but you can always fight with other players
- When a server dies the game no longer crashes - you will be just kicked to the main menu

## [0.1.0] - 2018-XX-XX

_0.1.0 was part of the legacy engine_

[unreleased]: https://gitlab.com/veloren/veloren/compare?from=v0.4.0&to=master
[0.0.4]: https://gitlab.com/veloren/veloren/compare?from=v0.3.0&to=v0.4.0
[0.0.3]: https://gitlab.com/veloren/veloren/compare?from=v0.2.0&to=v0.3.0
[0.0.2]: https://gitlab.com/veloren/veloren/compare?from=7d17f8b67a2a6d5aa00730f028cedc430fd5075a&to=v0.2.0
[0.0.1]: https://gitlab.com/veloren/game
