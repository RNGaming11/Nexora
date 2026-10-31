# Minecraft 1.21.1 port

This directory is the version-specific source of truth for the Nexora 1.21.1 port. It is deliberately separate from the root `research-lock.json`: the existing branch baseline still describes 26.3, and changing that lock in place would invalidate the current generated constants and local preparation.

## Verified target facts

The target is Java Edition 1.21.1:

| Fact | Value |
| --- | ---: |
| Release date | 2024-08-08 |
| Java runtime | 21 |
| Network protocol | 767 |
| World data version | 3955 |
| Resource pack format | 34 |
| Data pack format | 48 |
| Asset index | 17 |

The version-specific artifact URLs and SHA-1 values are recorded in [target.json](target.json). They include both client and server jars and Mojang's official client/server mappings. The jar hashes and mapping hashes must be checked against the downloaded bytes before use.

## Important compatibility difference

Minecraft 1.21.1 is an obfuscated release. Its metadata contains `client_mappings` and `server_mappings`; the existing `tools/version_port.py prepare` deliberately rejects versions containing those entries. Do not remove that guard as a workaround: the current class index and source-diff pipeline would compare obfuscated symbols against the deobfuscated 26.3 baseline and produce misleading results.

The dedicated `tools/prepare_1_21_1.py` path now applies official ProGuard mappings to both class inventories. `tools/proguard.py` parses class, field and method mappings, resolves member descriptors after reading the complete class map (so forward type references work), and `tools/class_index.py` emits semantic class/member names while normalizing descriptors and bytecode references. The raw obfuscated inventory is retained separately. The importer indexes only classes present in the corresponding Mojang mapping file, avoiding unrelated bundled libraries.

The remaining import and port steps are:

1. Resolve the version entry from Mojang's current `version_manifest_v2.json`; Mojang can republish version JSONs, so the metadata SHA-1 in `target.json` is an observed pin, not an eternal manifest identity.
2. Download and verify the client jar, server bundler, and both official mapping files against the hashes in the version JSON.
3. Unpack the server bundler and verify the extracted game jar.
4. Apply the correct side's Mojang mappings when producing class inventories and decompiled sources. Keep raw obfuscated names available as provenance; never overwrite the downloaded jars.
5. Run the 1.21.1 data generator using Java 21 and capture reports.
6. Generate version constants from the actual embedded client `version.json`; do not hand-edit `crates/horizon-core/src/version.rs`.
7. Diff mapped symbols and generated data against a 1.21.1-specific candidate baseline, then run the parity harnesses before treating any port as complete.

## Packet ID comparison

After generating `packet_ids.rs` from the 1.21.1 data-generator report, compare it with the current protocol inventory:

```powershell
python tools/compare_packet_ids.py
```

The tool writes `packet-id-diff.json` and prints counts for packets added, removed, and renumbered relative to the current 26.3 inventory. This is an inventory comparison only: matching names and IDs do not prove packet payload codecs or connection behavior are compatible.

## Typed packet codec coverage audit

After generating the target packet IDs, run:

```powershell
python tools/audit_packet_codec_coverage.py
```

This scans the existing Rust packet enums and reports which target packet constants already have typed packet codecs, grouped by state and direction. It writes `packet-codec-coverage.json`. A matching constant is only an initial reuse candidate: payload field layouts and semantics still need verification against 1.21.1 sources.

## Current status

- [x] Isolated working branch: `nexora/1.21.1-port-baseline`.
- [x] Version facts and official artifact/mapping hashes recorded.
- [x] Mapping-aware prepare and class-index pipeline (raw and mapped inventories are emitted separately).
- [x] Verified 1.21.1 jars, extracted server game jar, and data-generator reports.
- [ ] 1.21.1 generated constants and registries.
- [ ] Mapped source/class diff and port worklist.
- [ ] Gameplay parity implementation and tests.
- [ ] Playable client/server milestone.

No game code is claimed to be ported by this baseline commit. The first playable milestone must be demonstrated by launching the Rust client and joining a local 1.21.1 server, not by a successful Rust compile alone.
