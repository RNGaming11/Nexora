# Contributing to Nexora

Thanks for helping with Nexora. The active development branch for the Minecraft 1.21.1 port is
`nexora/1.21.1-port-baseline`.

## Port status and rules

The game workspace is still pinned to Minecraft 26.3 / protocol 777. The protocol 767 codecs for
Minecraft 1.21.1 are isolated and tested, but the live client session, generated registries and
gameplay data have not been ported yet. Do not change the pinned version constants or claim 1.21.1
support until the mapped class diff, generated data, network integration and parity tests are complete.

Keep changes focused. Preserve existing behavior unless the change explicitly replaces it. Do not
rename the internal `horizon-*` crate and binary identifiers as part of unrelated port work; they
are still referenced throughout the workspace and tooling.

## Build and verify

Requirements: Rust stable, Python 3.10+, and the JDK required by the version-preparation tools.

From the repository root:

```powershell
cargo check --workspace
.\tools\verify_1_21_1.ps1
```

The launcher is its own Cargo workspace:

```powershell
cd launcher
cargo check
```

## Continue the 1.21.1 port

Read [ports/1.21.1/README.md](ports/1.21.1/README.md) before changing protocol or version data.

After preparing the 1.21.1 artifacts, compare the mapped class inventory and generated reports
against the pinned baseline:

```powershell
python tools/prepare_1_21_1.py
python tools/version_port.py prepare 26.3
python tools/version_port.py diff 26.3 1.21.1
python tools/version_port.py report 26.3 1.21.1
```

Use the resulting `diff-26.3-1.21.1/report.md` as the port worklist. Port one coherent area at a
time, add regression tests, run the verifier, and check the GitHub Actions result before moving on.

## Pull requests

Target `nexora/1.21.1-port-baseline`. Include:
- The behavior being changed and why.
- Tests added or updated.
- The exact verification commands run.
- Any remaining limitation, especially where protocol 767 still falls back to 26.3 behavior.

Nexora is an unofficial project and is not affiliated with or endorsed by Mojang or Microsoft.
