<p align="center">
	<img alt="Veloren" src="https://raw.github.com/veloren/game/master/misc/logo.png">
</p>

[![Build Status](https://travis-ci.org/veloren/game.svg?branch=master)](https://travis-ci.org/veloren/game)
[![build status](https://gitlab.com/veloren/game/badges/master/build.svg)](https://gitlab.com/veloren/game)
[![coverage](https://gitlab.com/veloren/game/badges/master/coverage.svg)](https://gitlab.com/veloren/game/pipelines)

<p align="center">
	<img alt="A screenshot of Velore gameplay" src="https://raw.github.com/veloren/game/master/misc/screenshot1.png">
</p>

## Welcome To Veloren!

Veloren is a multiplayer voxel RPG written in Rust. Veloren takes inspiration from games such as Cube World, Minecraft and Dwarf Fortress. The game is currently under heavy development, but is playable.

## Useful Links

### [Current Status](status)

See here for a list of Veloren's features and what we're working on right now.

### [Releases](releases)

See here for a list of past releases.

### [Future Plans](future)

See here for information about Veloren's development roadmap.

### [How To Contribute](contribute)

See here for information on how you can get involved in shaping Veloren's future. You don't have to know how to program to contribute!

## Credit

Many thanks to everyone that has contributed to Veloren's development, provided ideas, crafted art, composed music, hunted bugs, created tools and supported the project.

## Provide a gitlab destroy_upgraded_shared_port_when_sender_still_active
If you have a spare computer or vhost you can help the veloren team by providing a gitlab runner to increase test speed for developers.
Follow the following steps on your machine. Keep in mind that this basically allows remote execution of any code on your machine.
1. Install gitlab runner on your host: https://docs.gitlab.com/runner/install/linux-repository.html
2. Follow all steps for comiling the project like descriped above. make sure you can compile the code as gitlab-runner user
3. install gcc crosscompiler for windows and zip
```bash
sudo apt-get install gcc-mingw-w64-x86-64 zip libssl-dev pkg-config cmake zlib1g-dev libncurses5-dev -y
rustup target add x86_64-pc-windows-gnu
```
(https://stackoverflow.com/a/39184296/4311928)
4. put in `~/.cargo/config` of gitlab-runner user:
```bash
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-gcc-ar"
```
5. get a Iphlpapi.dll from windows and put it here: `~/.multirust/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-pc-windows-gnu/lib`
	make sure only the I is capital. if you have no windows to get this file, ask in the discord chat
6. register your runner https://docs.gitlab.com/runner/register/
	- https://gitlab.com
	- take the token from: https://gitlab.com/veloren/game/settings/ci_cd
	- description: veloren <your user name>
	- tags: <none, just press enter>
	- executor: shell
7. check of your runner appears here: https://gitlab.com/veloren/game/settings/ci_cd
