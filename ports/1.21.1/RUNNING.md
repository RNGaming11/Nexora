# Nexora — Minecraft 1.21.1 port workspace

The 1.21.1 port is maintained on branch `nexora/master`. This is a development workspace, not a playable 1.21.1 launcher yet.

## Requirements

- Windows 10/11 with PowerShell
- Python 3.11 or newer
- JDK 21 (the `java` command must resolve to Java 21)
- Rust toolchain and Visual Studio C++ Build Tools for Rust builds and tests

Microsoft sign-in is not needed to prepare Mojang artifacts, generate protocol inventories, or run the port's automated tests. It is only needed for the launcher's normal online-mode Minecraft account flow. Do not add a fake client ID or reuse another launcher's credentials.

## Verify the existing baseline

From the repository root:

```powershell
.\tools\verify_1_21_1.ps1
```

This uses the current local checkout and runs the mapping and packet-codec Python tests, Rust formatting, core/protocol/world tests, and the workspace compile check. If the prepared data-generator packet report is present, it regenerates and audits the target packet inventory locally. It never pulls, commits, or pushes, so running verification cannot trigger remote workflows. The full log is written to `target/1.21.1-verification.log`.

The artifact preparation and mapping-aware class-index work are already implemented. Do not rerun preparation just because this document was opened.

## Prepare or refresh Mojang's official artifacts

Only when the artifact cache is missing or needs refreshing, run:

```powershell
py -3 --version
java -version
py -3 tools/prepare_1_21_1.py
```

Optional parallel indexing:

```powershell
py -3 tools/prepare_1_21_1.py --jobs 8
```

Force a fresh download:

```powershell
py -3 tools/prepare_1_21_1.py --force
```

The script resolves 1.21.1 from Mojang's live version manifest, verifies the client jar, server bundler, mappings and extracted server jar against official hashes, checks the version facts embedded in the client jar, applies the correct Mojang mappings to class inventories, and runs the vanilla data generator with Java 21.

## Output

By default, generated files are stored outside the repository:

```text
~/code/horizon-parity/versions/1.21.1/
```

Set `HORIZON_VERSIONS` to use another artifact-cache directory:

```powershell
$env:HORIZON_VERSIONS = "D:\Nexora-artifacts"
py -3 tools/prepare_1_21_1.py
```

The preparation report is `prepared.json`; version facts are `facts.json`; raw and mapped class inventories are stored separately; generated vanilla reports are under `datagen/reports`. Large downloaded and generated files intentionally stay out of Git.

## Current implementation boundary

Already implemented:
- Official 1.21.1 target facts, artifact/mapping pins, mapping-aware preparation, and mapped class inventories.
- Version-isolated protocol-767 IDs and codecs, including the feature-gated handshake, login, configuration, and partial play-state network path.
- Target-specific chunk/light, inventory/item-component, recipe, player-position, entity-motion, and movement packet adapters.
- Separate 1.21.1 constants, registries, item-component IDs, and a runtime resource bundle pipeline that leaves downloaded game data outside Git.
- A separate `horizon-client-1-21-1` binary target; the normal executable remains on 26.3.
- The shared Rust game, renderer, world, inventory, and gameplay systems are reused rather than rewritten.

Still incomplete:
- The 1.21.1 resource bundle still needs the remaining mob-data, entity-model-layer, and creative-tab exports wired into the existing Rust loaders.
- The protocol-767 play adapter still has packets that decode but are not yet integrated into gameplay, plus packets without verified target payload adapters. These must be addressed against the pinned target source and fixtures.
- Remaining raw item-component values need gameplay-level interpretation where required.
- The launcher still needs a complete 1.21.1 research lock and target-specific resource preparation/launch flow.
- End-to-end gameplay parity and a demonstrated join to a local vanilla 1.21.1 server are not complete.

The root workspace still defaults to Minecraft 26.3. Do not change its default version constants or claim 1.21.1 is playable based only on a successful compile. The verification script is local-only: it does not pull, commit, or push.
