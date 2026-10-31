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

This fast-forwards the current branch, runs the mapping and packet-codec Python tests, Rust formatting, protocol tests, and the workspace compile check. If the prepared data-generator packet report is present, it also regenerates and audits the target packet inventory. The full log is written to `target/1.21.1-verification.log`.

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

Already present:
- Official 1.21.1 target facts and artifact/mapping hash pins.
- Mapping-aware preparation and semantic class indexing.
- Verified client/server artifacts and vanilla data-generator reports.
- Version-isolated protocol-767 packet IDs and codecs for the previously missing packet types.
- Login and configuration dispatch adapters, kept separate from the active 26.3 protocol.

Still required:
- Generate and validate the 1.21.1 version constants and registries.
- Produce and review the mapped 26.3-to-1.21.1 class/source diff and ranked worklist.
- Select protocol 767 throughout the live client connection instead of the current protocol 777.
- Integrate 1.21.1 login, configuration and play-state dispatch into the live network state machine.
- Implement the registry-aware recipe serializers and remaining gameplay compatibility.
- Demonstrate a playable milestone by launching the Rust client and joining a local vanilla 1.21.1 server.

The root workspace still targets Minecraft 26.3. Do not change `crates/horizon-core/src/version.rs` by hand, remove the obfuscation guard in `tools/version_port.py`, or describe this branch as a playable 1.21.1 port. A successful compile is not proof of gameplay compatibility.
