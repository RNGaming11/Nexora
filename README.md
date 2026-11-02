# Nexora

Nexora is a Minecraft: Java Edition client and launcher project. This repository contains the Rust game client, launcher, server tools, tests and the version-port tooling.

> **Port status:** Minecraft 1.21.1 support is in progress and is not yet playable.

## Current branch status

This branch, `nexora/master`, is the working baseline for the Minecraft 1.21.1 port.

**Important:** the game itself is still pinned to Minecraft 26.3 / protocol 777. The protocol 767 (Minecraft 1.21.1) packet codecs are isolated and tested, but they are not yet connected to the live client session. The 1.21.1 registries, generated game data and gameplay compatibility are also unfinished. Do not treat the current build as a playable 1.21.1 client.

The launcher has Nexora branding and a blue/cyan theme. Advertising components and the old telemetry/anonymous player-count pipeline are not included in this branch. Optional Discord Rich Presence is a separate integration with Discord.

## Build the current baseline

Requirements: Rust stable and Python 3.10+. Resource preparation automatically selects the exact Java major for the Minecraft target (Java 21 for 1.21.1, Java 25 for 26.3), downloading Mojang's runtime into the external cache if needed.

From the repository root:

```powershell
cargo check --workspace
.\tools\verify_1_21_1.ps1
```

The launcher is a separate Cargo workspace:

```powershell
cd launcher
cargo check
```

## Continue the 1.21.1 port

Read [the 1.21.1 port notes](ports/1.21.1/README.md) first. They describe the generated packet inventory, the preparation requirements and the remaining integration work.

After preparing the 1.21.1 data, generate the mapped class diff and ranked worklist against the pinned baseline:

```powershell
python tools/prepare_1_21_1.py
python tools/version_port.py prepare 26.3
python tools/version_port.py diff 26.3 1.21.1
python tools/version_port.py report 26.3 1.21.1
```

The report is written to the external version store under `diff-26.3-1.21.1/report.md`. It identifies the classes and generated data that need porting; it does not itself make the game compatible.

## Project links

- [Source repository](https://github.com/RNGaming11/Nexora)
- [Current port branch](https://github.com/RNGaming11/Nexora/tree/nexora/master)
- [Releases](https://github.com/RNGaming11/Nexora/releases)
- [Issues](https://github.com/RNGaming11/Nexora/issues)
- [License](LICENSE)

Nexora is an unofficial project and is not affiliated with or endorsed by Mojang or Microsoft. Minecraft: Java Edition and its assets are not distributed in this repository.
