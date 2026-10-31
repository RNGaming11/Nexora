# Minecraft 1.21.1 port

This directory is the version-specific source of truth for the Nexora 1.21.1 port. It stays separate from the root `research-lock.json` and generated constants because the active workspace still targets Minecraft 26.3.

## Target facts

| Fact | Minecraft 1.21.1 |
| --- | ---: |
| Release date | 2024-08-08 |
| Java runtime | 21 |
| Network protocol | 767 |
| World data version | 3955 |
| Resource pack format | 34 |
| Data pack format | 48 |
| Asset index | 17 |

Official client/server artifacts and Mojang mapping hashes are pinned in [target.json](target.json).

## Completed preparation work

Minecraft 1.21.1 is obfuscated. The normal `tools/version_port.py prepare` path deliberately rejects releases with Mojang mapping files; do not remove that guard. The dedicated `tools/prepare_1_21_1.py` path handles this target and applies official ProGuard mappings to both class inventories. The mapping parser resolves member descriptors after reading the complete class map, and the class index emits semantic class/member names while retaining raw obfuscated names as provenance.

The preparation work already present on this branch includes:
- Verified client and server jars, extracted server game jar, and vanilla data-generator reports.
- Version facts and official artifact/mapping hash pins.
- Separate raw and mapped class inventories.
- Isolated 1.21.1 version constants exposed as `horizon_core::version_1_21_1`; tests verify they match `target.json` while the active 26.3 constants remain unchanged.
- Version-isolated protocol-767 packet IDs and codecs for the previously missing packet types.
- The generated `packet_ids.rs` inventory is imported by `horizon-protocol`; Rust tests compare every version-specific codec ID against the generated Mojang packet names so the hand-written dispatch table cannot silently drift.
- Login and configuration dispatch adapters, kept separate from the active 26.3 protocol.
- The 23 recipe serializer IDs are pinned in [`recipe-serializers.json`](recipe-serializers.json) from the official 1.21.1 `minecraft:recipe_serializer` registry, not a later-version packet enum. The preparation workflow regenerates this inventory from Mojang's report and checks it byte-for-byte. Vanilla serializer bodies are decoded and round-tripped for shaped, shapeless, special, cooking, stonecutting, and smithing recipes.
- The 57 item data-component IDs are pinned in [`data-component-ids.json`](data-component-ids.json) and exposed through a protocol-767 lookup table. Recipe slots report the exact unsupported component rather than interpreting it with the incompatible 26.3 table.
- The world block registry now has `BlockRegistry::from_nbt_with_data_version`, so a 1.21.1 caller can validate DataVersion 3955 without changing the active 26.3 constant (5023).
- Recipe slot data-component patches are explicitly rejected until their 1.21.1-specific codecs are implemented; the 26.3 component ID table is never reused.
- Built-in registry report parsing rejects duplicate IDs, gaps, negative/missing IDs, and IDs outside the contiguous range instead of silently constructing invalid lookup tables.

Large downloaded and generated artifacts intentionally stay outside Git. By default, the preparation cache is under `~/code/horizon-parity/versions/1.21.1/`; set `HORIZON_VERSIONS` to select another location.

## Generated 26.3-to-1.21.1 comparison

The preparation workflow builds a mapped class diff and ranked worklist from the official artifacts. The latest generated comparison records these version changes:

| Fact | Active 26.3 | Target 1.21.1 |
| --- | ---: | ---: |
| Java major | 25 | 21 |
| Protocol | 777 | 767 |
| World data version | 5023 | 3955 |
| Resource pack format | 97.1 | 34 |
| Data pack format | 121.0 | 48.0 |

Protocol inventory changes are substantial: play clientbound has 4 added, 24 removed, and 118 renumbered packets; play serverbound has 3 added, 14 removed, and 54 renumbered. The registry comparison records 13 added, 30 removed, and 40 changed registries. The mapped class comparison records 1,132 added, 4,246 removed, 589 moved, and 5,353 changed classes.

The highest-ranked blocker is the data-component registry: the target has a different 57-entry registry, with 69 current-version component types absent and 4 target-only types. This is why the port keeps its own component ID table and does not reuse the 26.3 item component IDs.

The complete machine-generated comparison contains 930 ranked work items and is uploaded as the `nexora-26.3-to-1.21.1-port-diff` Actions artifact.

## Next port stages

1. Generate and validate 1.21.1 registries from the verified vanilla artifacts. The version constants are isolated in `ports/1.21.1/version.rs` and exposed as `horizon_core::version_1_21_1`; do not hand-edit `crates/horizon-core/src/version.rs`, which describes the active 26.3 workspace.
2. Generate and review the mapped 26.3-to-1.21.1 class/source diff and ranked worklist. With both version caches prepared, run:

   ```powershell
   python tools/version_port.py prepare 26.3
   python tools/version_port.py diff 26.3 1.21.1
   python tools/version_port.py report 26.3 1.21.1
   ```

3. Complete protocol-767 play-state dispatch and connect version selection to the live network state machine. The current client still sends protocol 777 and uses 26.3 packet enums; the 1.21.1 codecs are not yet live.
4. Implement the registry-aware built-in recipe serializers and validate registry loading against 1.21.1 wire fixtures.
5. Port gameplay systems in dependency order, using the mapped source diff and parity tests.
6. Demonstrate the playable milestone by launching the Rust client and joining a local vanilla 1.21.1 server. A successful compile or packet-codec test alone is not proof of gameplay compatibility.

Do not change the active protocol constant to 767 as a shortcut: that would make the existing 26.3 client send a 1.21.1 handshake while still decoding 26.3 packets.

## Packet inventory and codec audit

When the 1.21.1 data-generator packet report is available, the tools below generate a target packet inventory and compare it to the active protocol:

```powershell
python tools/generate_packet_ids.py --report <path-to-packets.json> --out ports/1.21.1/packet_ids.rs --protocol-version 767 --version-name 1.21.1
python tools/compare_packet_ids.py
python tools/audit_packet_codec_coverage.py
```

The audit distinguishes ID-aligned reuse candidates, version-specific codecs, and missing codec candidates. Matching packet names or IDs do not prove matching payload layouts; `payload_compatibility_verified_count` remains zero until codecs have been checked against target source and wire fixtures.

Eighteen of the nineteen previously missing packet payloads have concrete codecs and round-trip tests. The remaining `update_recipes` payload is registry-dependent and uses an explicit `RecipeSerializer` interface. Do not guess its serializer body format: doing so can desynchronize the connection.

## Current status

- [x] Target facts and official artifact/mapping hash pins.
- [x] Mapping-aware preparation and class indexing.
- [x] Verified client/server artifacts and vanilla data-generator reports.
- [x] Version-isolated codecs for the previously missing packet payloads (18 concrete; `update_recipes` registry-dependent).
- [x] Protocol-767 login and configuration dispatch adapters.
- [x] Isolated 1.21.1 version constants, checked against the pinned target metadata.
- [ ] Generated 1.21.1 registries.
- [ ] Mapped source/class diff and ranked port worklist.
- [ ] Protocol-767 play-state integration in the live client.
- [x] Protocol-767 recipe serializer registry and vanilla serializer body codecs (component-bearing slots still fail closed).
- [ ] Complete 1.21.1 item data-component payload codecs for recipe slots and inventory packets.
- [ ] Gameplay parity.
- [ ] Playable client/server milestone.

Microsoft sign-in is not required for artifact preparation, protocol development, or automated tests. It remains required for the launcher's normal online-mode account flow. Keep the real authentication implementation intact; do not add a fake client ID or reuse another launcher's credentials.

## Preparing the full 1.21.1 data set

The repository includes a one-click GitHub Actions workflow named **Prepare Minecraft 1.21.1 port data**. Run it from the Actions tab using **Run workflow**. It installs Python and Java 21, downloads and verifies Mojang's official artifacts, runs the vanilla server data generator, applies official mappings, builds the mapped class inventories, and uploads the registry/packet/item/block/command reports and class diff as the `nexora-minecraft-1.21.1-port-data` artifact. It also installs Java 25 to prepare the active 26.3 baseline and, when that succeeds, uploads the mapped 26.3-to-1.21.1 comparison and ranked worklist as `nexora-26.3-to-1.21.1-port-diff`. The workflow does not commit downloaded Minecraft jars or generated bulk data into the repository.

## Verification

From the repository root on Windows PowerShell:

```powershell
.\tools\verify_1_21_1.ps1
```

The script fast-forwards the current branch, runs the Python mapping and packet-audit tests, checks Rust formatting, runs protocol tests, and checks the workspace build. If the prepared packet report is found at the default cache path or through `NEXORA_1_21_1_REPORT`, it also regenerates and audits the target packet inventory. Full output is saved to `target/1.21.1-verification.log`.

GitHub Actions runs the same core Python tests, formatting check, protocol tests, and workspace compile check on pushes and pull requests to `nexora/master`. The verification script's local results and GitHub Actions results must be checked before calling a build verified.
