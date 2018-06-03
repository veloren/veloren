# Welcome to Verloren

[![Build Status](https://travis-ci.org/veloren/game.svg?branch=master)](https://travis-ci.org/veloren/game)

## What is Verloren?
Verloren is a multiplayer voxel game inspired by Cube World. It aims to emulate the feel of Cube World while deviating in its features.

## Licensing and contribution

Verloren is an open-source community project licensed under the General Public License version 3. We gratefully welcome community contributions, both technical and editorial.

## Why so many crates?

Verloren is designed in a modular manner. The various crates (libraries, for those not acquainted with Rust) of the project try to make no assumptions about what they will be used by and how.

### 'Core' crates

#### `client`

The client crate. Used by client 'frontends' to manage all the low-level details of running a Verloren client like handling the server connection, entity physics predictions, terrain updates, etc.

#### `server`

The server crate. Like the `client` crate, it's used by server frontends to manage the low-level details of running a Verloren server like handling client connections, entity physics, terrain generation, world simulation, etc.

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

The `voxygen` crate contains the official 3D frontend for Verloren. It allows players to connect to servers, play as a character within those servers, and will eventually also support single player worlds via the network abstraction within the `network` crate (note: this isn't implemented yet). It uses the `client` crate. It uses the `server` crate.

#### `server-cli`

This crate is a simple command-line interface (CLI) server frontend. It allows the hosting of a server in a headless environment such as a dedicated server.

#### `server-gui` (not yet implemented)

This crate is a graphical user interface (GUI) server frontend. It allows the hosting of a server front an ordinary desktop PC with minimal effort and is designed for use by ordinary players wanting to host a server. It uses the `server` crate.

## Compilation

1. Install dependencies necessary for building

**For Arch Linux:**
```bash
$ pacman -S rust
$ rustup default nightly # Configure rust nightly compiler
$ pacman -U https://archive.archlinux.org/packages/s/sfml/sfml-2.4.2-5-x86_64.pkg.tar.xz #needed for now, because sfml is normaly 2.5 and csfml only 2.4
$ pacman -S csfml
```

**For Windows:**

Install Rust from [here](https://www.rust-lang.org/en-US/install.html)
```
rustup default nightly
```
Follow instructions from [here](https://github.com/jeremyletang/rust-sfml/wiki/How-to-use-rust-sfml-on-Windows) to install SFML/CSFML libraries.


2. Compile and run `worldtest`

```bash
(cd worldtest && cargo run)
```

3. Compile and run `server-cli`

```bash
(cd server-cli && cargo run)
```

4. Compile and run `frontend`

```bash
(cd frontend && cargo run)
```

## Task list

There are a variety of things that need doing at the moment. You can help move the project forwards by contributing!

- [ ] Get things ready for the 0.1.0 release
	- [ ] Make `server` understand that players have a position in the world
	- [ ] Make `server` generate chunks when players approach them
	- [ ] Make `server` send nearby chunks over the network to `client` (only when requested)
	- [ ] Add volume rendering to `voxygen`
- [ ] Fix the *major* slowdown in `server` whereby the entire thing locks up when a world simulation update is occuring
	- [ ] Make worldsim internally concurrent and externally concurrent (i.e: other things can still call functions on it while a world simulation update is occuring)
- [ ] Investigate how best to build a menu GUI for `voxygen`
	- [ ] Investigate font renderering vs texture rendering
	- [ ] Investigate 2D texture rendering
