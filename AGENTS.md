# Nexora: agent guide

Nexora is the product name. This repository is the Rust game client, launcher, server tools and port tooling.
The active Minecraft 1.21.1 port branch is `nexora/master`.

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [ports/1.21.1/README.md](ports/1.21.1/README.md) before
changing version or protocol code.

## Rules

- **Do not claim the 1.21.1 port is playable.** The root game workspace is still pinned to Minecraft
  26.3 / protocol 777. Protocol 767 codecs are isolated; the live network session, generated registries,
  game data and gameplay are not yet ported.
- **Do not edit pinned version constants by hand.** Generate facts from the actual target client
  `version.json`; keep the 26.3 baseline intact until the mapped diff and port worklist are reviewed.
- **No ads, analytics, telemetry or player-count heartbeat.** The old telemetry/anonymous heartbeat
  pipeline is not part of this branch. Do not add it back or transmit player identity, install IDs,
  play sessions, crash reports or performance reports to a project-operated service. Optional local
  integrations must remain explicit and separate.
- **Keep visible product branding Nexora.** Do not add Horizon product names, old Horizon website
  links, old release links or old server auto-add behavior. Internal `horizon-*` crate names, file
  formats and compatibility paths remain unchanged for now; renaming those is a separate migration.
- **Do not commit Mojang files.** Never add game jars, assets, decompiled source, prepared resources
  or anything from `.local/`. `tools/check_public_tree.py` checks the public tree.
- **Protect real player data.** Run game and launcher tests with fresh temporary data directories.
  Never write to the user's real Minecraft folder, launcher data, keychain or account files.
- **Keep changes focused.** Preserve existing behavior unless the task explicitly changes it. Add
  regression tests for behavior changes, and do not hide compiler errors with broad allow attributes.
- **Check the active branch and recent commits before editing.** Keep port changes on
  `nexora/master`; do not force-push or rewrite its history.

## Workspace map

The root Cargo workspace contains the game and protocol crates. `launcher/` is a separate Cargo
workspace. Crate names remain `horizon-*` internally while the product is rebranded.

| Area | Location |
|---|---|
| Game client and live network session | `crates/horizon-client/` |
| Protocol framing and packet codecs | `crates/horizon-protocol/` |
| Minecraft version constants | `crates/horizon-core/src/version.rs` |
| World data, registries and generation | `crates/horizon-world/`, `crates/horizon-items/` |
| Launcher UI, install/update and account flows | `launcher/src/` |
| Nexora theme and design tokens | `crates/horizon-theme/` |
| Version preparation and port analysis | `tools/`, `ports/1.21.1/` |
| Tests and parity harnesses | `tests/`, `tools/`, crate-local tests |

There is no telemetry crate or telemetry server in this branch.

## Build and verify

From the repository root:

```powershell
cargo check --workspace
.\tools\verify_1_21_1.ps1
```

The launcher has its own workspace:

```powershell
cd launcher
cargo check
```

## Port workflow

After preparing the 1.21.1 artifacts, compare them with the pinned baseline:

```powershell
python tools/prepare_1_21_1.py
python tools/version_port.py prepare 26.3
python tools/version_port.py diff 26.3 1.21.1
python tools/version_port.py report 26.3 1.21.1
```

Use `diff-26.3-1.21.1/report.md` as the ranked worklist. Port one coherent subsystem at a time:
packet/session state, registries and data, then gameplay. Run tests and GitHub Actions after each
coherent change. A successful workspace compile does not prove protocol or gameplay compatibility.
