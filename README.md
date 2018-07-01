# Welcome to Veloren

[![Build Status](https://travis-ci.org/veloren/game.svg?branch=master)](https://travis-ci.org/veloren/game)
[![build status](https://gitlab.com/veloren/game/badges/master/build.svg)](https://gitlab.com/veloren/game)
[![coverage](https://gitlab.com/veloren/game/badges/master/coverage.svg)](https://gitlab.com/veloren/game/pipelines)

<p align="center">
	<img alt="Veloren" src="https://raw.github.com/veloren/game/master/misc/screenshot1.png">
</p>

## What is Veloren?
Veloren is a multiplayer voxel game inspired by Cube World. It aims to emulate the feel of Cube World while deviating in its features.

## Licensing and contribution

Veloren is an open-source community project licensed under the General Public License version 3. We gratefully welcome community contributions, both technical and editorial.

## Why so many crates?

Veloren is designed in a modular manner. The various crates (libraries, for those not acquainted with Rust) of the project try to make no assumptions about what they will be used by and how.

### 'Core' crates

#### `client`

The client crate. Used by client 'frontends' to manage all the low-level details of running a Veloren client like handling the server connection, entity physics predictions, terrain updates, etc.

#### `server`

The server crate. Like the `client` crate, it's used by server frontends to manage the low-level details of running a Veloren server like handling client connections, entity physics, terrain generation, world simulation, etc.

#### `world`

This crate provides code that relates to large-scale world generation and simulation. It contains code that manages weather systems, the virtual economy, quest generation, civilisation simulation, etc. Currently, it is only used by the `server` crate, but we're keeping it separate since we anticipate that it may be used by third-party tools.

#### `region`

This crate provides code that relates to local world simulation (i.e: that which occurs close to a player). It contains code that manages entity physics, chunk updates, NPC AI, etc. It is used by both the `client` and `server` crates.

#### `network`

This crate provides code to manage networking on both the client and server ends. It includes things like connection management, packet serialization/deserialization, etc. It is used by both the `client` and `server` crates.

### 'Frontend' crates

#### `headless`

This crate is a simple 'headless' (i.e: chat only) client frontend that connects to a server and allows the player to send and receive chat messages without having a physical character in the world. It uses the `client` crate.

#### `voxygen`

The `voxygen` crate contains the official 3D frontend for Veloren. It allows players to connect to servers, play as a character within those servers, and will eventually also support single player worlds via the network abstraction within the `network` crate (note: this isn't implemented yet). It uses the `client` crate. It uses the `server` crate.

#### `server-cli`

This crate is a simple command-line interface (CLI) server frontend. It allows the hosting of a server in a headless environment such as a dedicated server.

#### `server-gui` (not yet implemented)

This crate is a graphical user interface (GUI) server frontend. It allows the hosting of a server front an ordinary desktop PC with minimal effort and is designed for use by ordinary players wanting to host a server. It uses the `server` crate.

## Compilation

1. Install Rust

	**Arch Linux**

	```bash
	$ sudo pacman -S rustup
	$ rustup default nightly # Configure rust nightly compiler
	```

	**Ubuntu**

	```bash
	$ su <user> # log in to user which should build the project
	$ curl https://sh.rustup.rs -sSf | sh
	$ # select 2) Customize installation
	$ # select nightly toolchain, and allow modify PATH variable
	$ # proceed with installation
	$ # open a new terminal and make sure that you can type:
	$ cargo --version # if it cannot find it, look at the ~/.profile file and conside moving the PATH variable at the end of ~/.bashrc, open a new terminal and try again
	$ rustup default nightly # Configure rust nightly compiler
	```

	**Windows**

	Install Rust from [here](https://www.rust-lang.org/en-US/install.html)

	```
	> rustup default nightly
	```

2. Install SFML

	**Arch Linux**

	```bash
	$ sudo pacman -U https://archive.archlinux.org/packages/s/sfml/sfml-2.4.2-5-x86_64.pkg.tar.xz #needed for now, because sfml is normaly 2.5 and csfml only 2.4
	$ sudo pacman -S csfml
	```

	**Windows**

	Follow instructions from [here](https://github.com/jeremyletang/rust-sfml/wiki/How-to-use-rust-sfml-on-Windows) to install SFML/CSFML libraries.


3. Compile and run a backend

	Currently, the only officially supported backend is `server-cli`.

	```bash
	( cd server-cli && RUST_LOG=cargo=warning,server=debug,server-cli=debug,common=debug,region=debug,world=debug cargo run)
	```

4. Compile and run a frontend

	Currently, the only officially supported frontend is `voxygen`.

	```bash
	( cd voxygen && RUST_LOG=cargo=warning,client=debug,voxygen=debug,common=debug,region=debug,world=debug cargo run)
	```

## Provide a gitlab destroy_upgraded_shared_port_when_sender_still_active
If you have a spare computer or vhost you can help the veloren team by providing a gitlab runner to increase test speed for developers.
Follow the following steps on your machine. Keep in mind that this basically allows remote execution of any code on your machine.
1. Follow all steps for comiling the project like descriped above. make sure you can compile the code as gitlab-runner user
2. install gcc crosscompiler for windows and zip `sudo apt-get install gcc-mingw-w64-x86-64 zip -y`
(https://stackoverflow.com/a/39184296/4311928)
3. put in `~/.cargo/config` of gitlab-runner user:
```bash
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-gcc-ar"
```
4. get a Iphlpapi.dll from windows and put it here: ~/.multirust/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-pc-windows-gnu/lib
	make sure only the I is capital. if you have no windows to get this file, ask in the discord chat
5. Install gitlab runner on your host: https://docs.gitlab.com/runner/install/linux-repository.html
6. register your runner https://docs.gitlab.com/runner/register/
	- https://gitlab.com
	- take the token from: https://gitlab.com/veloren/game/settings/ci_cd
	- description: veloren <your user name>
	- tags: <none, just press enter>
	- executor: shell
7. check of your runner appears here: https://gitlab.com/veloren/game/settings/ci_cd
