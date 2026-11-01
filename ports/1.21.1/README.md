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
- The 57 item data-component IDs are pinned in [`data-component-ids.json`](data-component-ids.json) and exposed through a protocol-767 lookup table. Dedicated 1.21.1 item-stack, template, and delimited-slot codecs use those IDs. The version adapter now has bounded payload readers for all 57 component IDs, including the Adventure-mode block predicates for `can_place_on` and `can_break`; complex components that are not interpreted by gameplay code are retained as raw bytes.
- The world block registry now has `BlockRegistry::from_nbt_with_data_version`, so a 1.21.1 caller can validate DataVersion 3955 without changing the active 26.3 constant (5023).
- Recipe slot decoding now uses the 1.21.1 item-stack codec. Component patches are parsed with target IDs, component removals are validated against the target registry, and uninterpreted known components retain their version-specific bytes. Unknown component IDs still fail closed. The 26.3 ID table is never used for these slots.
- Built-in registry report parsing rejects duplicate IDs, gaps, negative/missing IDs, and IDs outside the contiguous range instead of silently constructing invalid lookup tables.
- A compact 1.21.1 built-in registry inventory generator and a version/DataVersion-checked Rust loader are in place. The preparation workflow emits all 78 registry name tables (5,825 IDs) as a separate artifact instead of committing another large generated data file.

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

1. The 1.21.1 executable deliberately reuses the existing 26.3 Rust client, renderer, world, inventory, and gameplay implementation; both binaries compile the same source. Only version-specific constants, protocol adapters, and compatibility data are selected by the `minecraft-1-21-1` feature. Preparation now reuses the existing vanilla gameplay/block-behaviour exporters, copied and adjusted only for the 1.21.1 mapped Java API, to supply exact target-version inputs to the same Rust loaders. It builds the official client resources, registry inventory, block facts/tags/model-state schemas, heightmap predicates, gameplay facts and block behaviours. The CI validator loads the assets through `Assets::load` and the renderer's `reload` path. The remaining resource inputs must be ported through the same existing exporter pipeline (collision/occlusion, motion, items, entities, mob data, creative data, and related tables); no second game engine is being written. The version constants remain isolated in `ports/1.21.1/version.rs` and exposed through the selected `horizon_core::version`; do not hand-edit `crates/horizon-core/src/version.rs`, which describes the default 26.3 profile.
2. Generate and review the mapped 26.3-to-1.21.1 class/source diff and ranked worklist. With both version caches prepared, run:

   ```powershell
   python tools/version_port.py prepare 26.3
   python tools/version_port.py diff 26.3 1.21.1
   python tools/version_port.py report 26.3 1.21.1
   ```

3. Expand the feature-gated protocol-767 live client path. The `minecraft-1-21-1` build selects the target version constants and uses separate handshake, login, configuration, play dispatch, chunk/light conversion, and item-slot codecs. It now routes container contents/slots, equipment, container clicks, and creative-slot updates through the target item-component IDs. This is still a partial play-state adapter: unsupported gameplay packets must be implemented or proven wire-compatible before normal server play is reliable.
4. Bridge the decoded 1.21.1 recipe definitions into gameplay and the recipe-book UI; the live network path now safely decodes the target's recipe synchronization packet.
5. Backport only the 1.21.1 API/protocol differences into the existing shared Rust systems, using the mapped source diff and parity tests. Keep the 26.3 implementation as the base; do not duplicate or rewrite the game systems.
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

All nineteen previously missing packet payloads now have target-specific codecs. The `update_recipes` play packet is decoded through the pinned 23-entry 1.21.1 serializer registry and consumed safely by the live network loop. Its definitions are not yet applied to the game's newer recipe-book UI, so recipe-book behavior remains a gameplay task. Unknown serializers or malformed recipe bodies fail closed rather than risking packet-stream desynchronization.

## Current status

- [x] Target facts and official artifact/mapping hash pins.
- [x] Mapping-aware preparation and class indexing.
- [x] Verified client/server artifacts and vanilla data-generator reports.
- [x] Version-isolated codecs for the previously missing packet payloads, including protocol-767 login/configuration and recipe dispatch (23 vanilla recipe serializer IDs; recipe slots use the version-scoped item codec).
- [x] Isolated 1.21.1 version constants, checked against the pinned target metadata.
- [x] Pinned all 57 target data-component IDs and verified them against Mojang's data-generator report.
- [x] Added version-scoped 1.21.1 item-stack, template, and delimited-slot codecs. Bounded readers cover all 57 target component IDs; the `can_place_on` and `can_break` block-predicate payloads now have round-trip and truncated-input tests. Complex components are preserved raw where gameplay does not yet need their fields.
- [x] Added a compact built-in registry inventory generator, strict Rust loader, and preparation artifact for the target's 78 registries / 5,825 IDs.
- [x] Version-aware block registry DataVersion validation without changing the active 26.3 constant.
- [x] Generated the mapped 26.3-to-1.21.1 class diff and 930-item ranked worklist.
- [x] Generate the core 1.21.1 runtime bundle: official client assets/data, built-in registry inventory, block facts/tags, model-state schemas, heightmap predicates, gameplay facts, and block behaviours. The gameplay/behaviour exporters are backports of the existing vanilla exporters, not replacement game logic; CI checks their registry coverage.
- [x] Export exact 1.21.1 collision/occlusion tables, movement properties, entity dimensions/categories, and the default entity attribute catalog through the mapped official server jar.
- [x] Use the official item data-generator report and pinned 57-entry data-component registry as the target runtime item inputs.
- [ ] Backport the remaining mob-data, entity-model-layer, and creative-tab exporters to 1.21.1 and feed their output into the existing Rust loaders.
- [x] Connect protocol-767 container content/slot/equipment and merchant-offer decoding, plus container-click/creative-slot encoding, to the live client path using the 1.21.1 item/component codecs.
- [x] Route protocol-767's dedicated experience-orb spawn packet into the shared orb simulation, preserving its server-provided position and XP value.
- [x] Apply protocol-767 recipe-book initialization/add/remove/highlight/settings packets to the shared UI, and encode recipe placement with the target's recipe-identifier field.
- [x] Verify serverbound recipe-book settings and villager trade selection against the target packet IDs and payload layouts.
- [x] Add live hotbar selection and horse-inventory packet adaptation into existing gameplay handlers.
- [x] Add a separately named `horizon-client-1-21-1` binary target; the normal `horizon-client` target still uses the 26.3 profile.
- [x] Build and package the separate 1.21.1 executable beside the 26.3 executable; the launcher selects the executable from the installed build's `research-lock.json`.
- [ ] Ship a 1.21.1-specific research lock and preparer/runtime-resource pipeline through the launcher so it can prepare and launch that target without relying on prebuilt local resources.
- [x] Add a feature-gated protocol-767 connection state machine with version-specific handshake, login, configuration, chunk/light conversion, and play dispatch.
- [ ] Expand remaining play-state packet coverage; several target packets are still explicitly reported as requiring gameplay integration.
- [ ] Implement gameplay-level handling for preserved raw component values and verify the target component data against live server fixtures.
- [ ] Gameplay parity.
- [ ] Playable client/server milestone.

Microsoft sign-in is not required for artifact preparation, protocol development, or automated tests. It remains required for the launcher's normal online-mode account flow. Keep the real authentication implementation intact; do not add a fake client ID or reuse another launcher's credentials.

## Building the separate client profile

The ordinary client remains the default 26.3 target. Build the isolated 1.21.1 executable without changing that default:

```powershell
cargo build -p horizon-client --bin horizon-client-1-21-1 --features minecraft-1-21-1
```

The executable is written to `target/debug/horizon-client-1-21-1.exe` on Windows (or the platform equivalent). A normal `cargo run -p horizon-client` still uses the 26.3 profile. This separate executable is a development target; the 1.21.1 gameplay port is not yet complete or advertised as playable.

## Preparing the full 1.21.1 data set

The repository includes a one-click GitHub Actions workflow named **Prepare Minecraft 1.21.1 port data**. Run it from the Actions tab using **Run workflow**. It installs Python and Java 21, downloads and verifies Mojang's official artifacts, runs the vanilla server data generator, applies official mappings, builds the mapped class inventories, and uploads the registry/packet/item/block/command reports and class diff as the `nexora-minecraft-1.21.1-port-data` artifact. It also installs Java 25 to prepare the active 26.3 baseline and, when that succeeds, uploads the mapped 26.3-to-1.21.1 comparison and ranked worklist as `nexora-26.3-to-1.21.1-port-diff`. The workflow does not commit downloaded Minecraft jars or generated bulk data into the repository.

## Verification

From the repository root on Windows PowerShell:

```powershell
.\tools\verify_1_21_1.ps1
```

The script uses the current local checkout, runs the Python mapping, packet-audit, and version-port tests, checks Rust formatting, runs core version, protocol, and world registry tests, and checks the workspace build. If the prepared packet report is found at the default cache path or through `NEXORA_1_21_1_REPORT`, it regenerates and audits the target packet inventory locally. It does not pull, commit, or push. Full output is saved to `target/1.21.1-verification.log`.

The repository's GitHub Actions workflows are currently disabled to prevent any metered runner usage. Verification must therefore be run locally on Windows; do not re-enable Actions or dispatch a workflow as part of this $0-only development process.
