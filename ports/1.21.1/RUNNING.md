# Nexora — Minecraft 1.21.1 port workspace

The 1.21.1 port is isolated on branch `nexora/1.21.1-port-baseline`. Do not run the existing 26.3 preparation command for this target: it intentionally rejects obfuscated releases.

## Requirements

- Windows 10/11, PowerShell
- Python 3.11 or newer
- JDK 21 (the `java` command must resolve to Java 21)
- Rust toolchain and the Visual Studio C++ Build Tools are still needed later for compiling Nexora; they are not needed for the artifact-preparation step.

## Prepare the official 1.21.1 artifacts

From the repository root in PowerShell:

```powershell
py -3 --version
java -version
py -3 tools/prepare_1_21_1.py
```

Optional: set the number of parallel indexing workers:

```powershell
py -3 tools/prepare_1_21_1.py --jobs 8
```

To force re-download of the artifacts:

```powershell
py -3 tools/prepare_1_21_1.py --force
```

The script resolves the 1.21.1 entry from Mojang's live version manifest, verifies every downloaded file against the SHA-1 in that official metadata, extracts the server game jar, checks the version facts embedded in the client jar, runs the vanilla data generator with Java 21, validates both mapping files, and records an inventory of the raw obfuscated classes.

## Output

By default, files are stored outside the repository in:

```text
~/code/horizon-parity/versions/1.21.1/
```

Set `HORIZON_VERSIONS` to choose another artifact-cache directory. For example:

```powershell
$env:HORIZON_VERSIONS = "D:\Nexora-artifacts"
py -3 tools/prepare_1_21_1.py
```

The preparation report is `prepared.json`; version facts are `facts.json`; the raw class inventory is `classes-obfuscated.json`; generated vanilla reports are under `datagen/reports`. These large downloaded/generated files are intentionally not committed.

## Current limitations — read before treating this as playable

This command prepares and verifies the official game inputs. It does **not** yet remap the obfuscated class inventory to Mojang's named symbols, regenerate Nexora's version constants, port missing gameplay systems, or produce a runnable 1.21.1 client. The current root workspace still targets its existing 26.3 baseline. Do not change `crates/horizon-core/src/version.rs` by hand or try to launch this branch as a finished 1.21.1 game.

The next engineering stages are mapping-aware symbol remapping, 1.21.1 protocol/data generation, parity harness adaptation, and implementation/testing of the missing client and server behavior. A playable milestone is only reached when the Rust client launches and successfully joins a local vanilla 1.21.1 server.
