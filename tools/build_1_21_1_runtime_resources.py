#!/usr/bin/env python3
"""Build a local, version-pinned Minecraft 1.21.1 runtime resource directory.

The client jar, extracted server jar, and resource bundle remain in the
external HORIZON_VERSIONS cache. This tool never commits or uploads game data.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import tempfile
import zipfile

try:
    from . import (
        generate_1_21_1_block_facts,
        generate_1_21_1_model_states,
        generate_builtin_registry_inventory,
    )
except ImportError:
    import generate_1_21_1_block_facts
    import generate_1_21_1_model_states
    import generate_builtin_registry_inventory

VERSION = "1.21.1"
DATA_VERSION = 3955
PROTOCOL_VERSION = 767
GENERATOR = "tools/build_1_21_1_runtime_resources.py"
BLOCK_TAG_PREFIX = "data/minecraft/tags/block/"


def digest(path: Path, algorithm: str) -> str:
    value = hashlib.new(algorithm)
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def safe_parts(name: str) -> tuple[str, ...]:
    if "\\" in name:
        raise ValueError(f"invalid archive path: {name!r}")
    path = PurePosixPath(name)
    if path.is_absolute() or not path.parts or any(
        part in ("", ".", "..") for part in path.parts
    ):
        raise ValueError(f"unsafe archive path: {name!r}")
    return path.parts


def read_archive_resources(jar: Path) -> dict[str, bytes]:
    result = {}
    with zipfile.ZipFile(jar) as archive:
        for info in archive.infolist():
            name = info.filename
            if info.is_dir() or not (
                name.startswith("assets/") or name.startswith("data/") or name == "pack.png"
            ):
                continue
            safe_parts(name)
            if name in result:
                raise ValueError(f"{jar}: duplicate archive entry {name}")
            result[name] = archive.read(info)

    if not any(name.startswith("assets/minecraft/") for name in result):
        raise ValueError(f"{jar}: missing vanilla Minecraft assets")
    if not any(name.startswith("data/minecraft/") for name in result):
        raise ValueError(f"{jar}: missing vanilla Minecraft data")
    return result


def read_block_tags(jar: Path) -> dict[str, str]:
    result = {}
    with zipfile.ZipFile(jar) as archive:
        for info in archive.infolist():
            name = info.filename
            if info.is_dir() or not name.startswith(BLOCK_TAG_PREFIX) or not name.endswith(".json"):
                continue
            safe_parts(name)
            key = "minecraft:" + name[len(BLOCK_TAG_PREFIX):-5]
            if key in result:
                raise ValueError(f"{jar}: duplicate block tag {key}")
            raw = archive.read(info).decode("utf-8")
            json.loads(raw)
            result[key] = raw

    if not result:
        raise ValueError(f"{jar}: no block tags found under {BLOCK_TAG_PREFIX}")
    return dict(sorted(result.items()))


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, separators=(",", ":"), ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def install_archive_resources(resources: dict[str, bytes], root: Path) -> tuple[int, int]:
    total_bytes = 0
    for name, data in sorted(resources.items()):
        destination = root.joinpath(*safe_parts(name))
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)
        total_bytes += len(data)
    return len(resources), total_bytes


def install_items(reports: Path, root: Path) -> int:
    source = reports / "minecraft" / "components" / "item"
    if not source.is_dir():
        # Minecraft 1.21.1 has an item registry report, not the later
        # minecraft/components/item report tree. Preserve the report without
        # pretending that it contains per-item component defaults.
        report = reports / "items.json"
        if report.is_file():
            destination = root / "items-report.json"
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(report, destination)
        return 0

    items = {}
    for path in sorted(source.rglob("*.json")):
        name = "minecraft:" + path.relative_to(source).with_suffix("").as_posix()
        document = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(document, dict) or not isinstance(document.get("components"), dict):
            raise ValueError(f"{path}: no item components object")
        items[name] = document["components"]

    if not items:
        raise ValueError("vanilla item component report is empty")
    write_json(root / "items.json", items)
    return len(items)


def install_biome_parameters(reports: Path, root: Path) -> int:
    source = reports / "biome_parameters" / "minecraft"
    if not source.is_dir():
        # The 1.21.1 client jar already contains the target's worldgen/biome
        # data. The quantized biome_parameters report is not emitted by this
        # version's data generator.
        return 0

    count = 0
    for path in sorted(source.rglob("*.json")):
        destination = root / "biome-parameters" / path.relative_to(source)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1

    if not count:
        raise ValueError("vanilla biome parameter report is empty")
    return count


def is_managed_output(output: Path) -> bool:
    manifest_path = output / "horizon-resources.json"
    if not manifest_path.is_file():
        return False
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return False
    return (
        manifest.get("generator") == GENERATOR
        and manifest.get("minecraft_version") == VERSION
        and manifest.get("data_version") == DATA_VERSION
    )


def build(
    client_jar: Path,
    server_game_jar: Path,
    reports: Path,
    output: Path,
    heightmap_predicates: dict | None = None,
    gameplay_facts: Path | None = None,
    block_behaviours: Path | None = None,
    entity_types: Path | None = None,
    cached_collision: Path | None = None,
    occlusion: Path | None = None,
    attributes: Path | None = None,
    movement_properties: Path | None = None,
) -> dict:
    client_jar = client_jar.resolve()
    server_game_jar = server_game_jar.resolve()
    reports = reports.resolve()
    output = output.resolve()

    if not client_jar.is_file() or not server_game_jar.is_file():
        raise ValueError("verified Minecraft 1.21.1 client and extracted server jars are required")
    if not reports.is_dir():
        raise ValueError(f"missing data generator report directory: {reports}")
    for name in ("registries.json", "commands.json", "blocks.json"):
        if not (reports / name).is_file():
            raise ValueError(f"missing Mojang data generator report: {reports / name}")
    if output in (Path(output.anchor), client_jar.parent, server_game_jar.parent):
        raise ValueError("refusing unsafe runtime resource output directory")
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists() and not is_managed_output(output):
        raise ValueError(f"refusing to replace an unrecognized output directory: {output}")

    client_resources = read_archive_resources(client_jar)
    client_tags = read_block_tags(client_jar)
    server_tags = read_block_tags(server_game_jar)
    if client_tags != server_tags:
        raise ValueError("client and server block tag resources disagree")

    stage = Path(tempfile.mkdtemp(prefix=output.name + ".staging-", dir=output.parent))
    try:
        resource_count, resource_bytes = install_archive_resources(client_resources, stage)

        registries_report = json.loads((reports / "registries.json").read_text(encoding="utf-8"))
        commands_report = json.loads((reports / "commands.json").read_text(encoding="utf-8"))
        write_json(stage / "registries.json", registries_report)
        write_json(stage / "commands.json", commands_report)
        write_json(stage / "block-tags.json", client_tags)

        inventory = generate_builtin_registry_inventory.build(
            reports / "registries.json", VERSION, DATA_VERSION
        )
        write_json(stage / "builtin-registry-ids.json", inventory)

        block_facts = generate_1_21_1_block_facts.build(
            reports / "blocks.json", DATA_VERSION
        )
        if gameplay_facts is None or not gameplay_facts.is_file():
            raise ValueError("official 1.21.1 gameplay facts are required for runtime resources")
        gameplay = json.loads(gameplay_facts.read_text(encoding="utf-8"))
        gameplay_states = gameplay.get("states") if isinstance(gameplay, dict) else None
        if not isinstance(gameplay_states, list) or len(gameplay_states) != len(block_facts["states"]):
            raise ValueError("1.21.1 gameplay facts do not match the block-state registry")
        for index, entry in enumerate(gameplay_states):
            if not isinstance(entry, dict) or entry.get("id") != index:
                raise ValueError(f"1.21.1 gameplay facts are malformed at state {index}")
        shutil.copyfile(gameplay_facts, stage / "gameplay-facts.json")

        if block_behaviours is None or not block_behaviours.is_file():
            raise ValueError("official 1.21.1 block behaviour facts are required for runtime resources")
        behaviours = json.loads(block_behaviours.read_text(encoding="utf-8"))
        behaviour_blocks = behaviours.get("blocks") if isinstance(behaviours, dict) else None
        behaviour_states = behaviours.get("states") if isinstance(behaviours, dict) else None
        expected_blocks = list(dict.fromkeys(row[0] for row in block_facts["states"]))
        if (
            not isinstance(behaviour_blocks, list)
            or not isinstance(behaviour_states, list)
            or len(behaviour_blocks) != len(expected_blocks)
            or len(behaviour_states) != len(block_facts["states"])
        ):
            raise ValueError("1.21.1 block behaviour facts do not match the block registry")
        for index, (entry, expected_name) in enumerate(zip(behaviour_blocks, expected_blocks)):
            if not isinstance(entry, dict) or entry.get("name") != expected_name:
                raise ValueError(f"1.21.1 block behaviour block order differs at block {index}")
        for index, entry in enumerate(behaviour_states):
            if not isinstance(entry, list) or len(entry) != 9:
                raise ValueError(f"1.21.1 block behaviour state {index} is malformed")
        shutil.copyfile(block_behaviours, stage / "block-behaviours.json")

        if entity_types is None or not entity_types.is_file():
            raise ValueError("official 1.21.1 entity type facts are required for runtime resources")
        entity_facts = json.loads(entity_types.read_text(encoding="utf-8"))
        expected_entity_types = inventory["registries"].get("minecraft:entity_type")
        if (
            not isinstance(entity_facts, dict)
            or not isinstance(expected_entity_types, list)
            or set(entity_facts) != set(expected_entity_types)
        ):
            raise ValueError("1.21.1 entity type facts do not match the built-in entity registry")
        for name, entry in entity_facts.items():
            if not isinstance(entry, dict) or any(
                not isinstance(entry.get(field), (int, float))
                for field in ("width", "height", "eye_height")
            ):
                raise ValueError(f"1.21.1 entity type facts are malformed for {name}")
        shutil.copyfile(entity_types, stage / "entity-types.json")

        if (
            cached_collision is None
            or not cached_collision.is_file()
            or occlusion is None
            or not occlusion.is_file()
        ):
            raise ValueError("official 1.21.1 collision and occlusion tables are required for runtime resources")
        if cached_collision.stat().st_size < 16 or occlusion.stat().st_size < 16:
            raise ValueError("1.21.1 collision and occlusion tables are incomplete")
        shutil.copyfile(cached_collision, stage / "cached-collision.nbt")
        shutil.copyfile(occlusion, stage / "occlusion.nbt")

        if attributes is None or not attributes.is_file():
            raise ValueError("official 1.21.1 entity attribute catalog is required for runtime resources")
        attribute_catalog = json.loads(attributes.read_text(encoding="utf-8"))
        definitions = attribute_catalog.get("attributes") if isinstance(attribute_catalog, dict) else None
        suppliers = attribute_catalog.get("entities") if isinstance(attribute_catalog, dict) else None
        expected_attributes = inventory["registries"].get("minecraft:attribute")
        if (
            attribute_catalog.get("data_version") != DATA_VERSION
            or not isinstance(definitions, list)
            or not isinstance(suppliers, dict)
            or not isinstance(expected_attributes, list)
            or {entry.get("id") for entry in definitions if isinstance(entry, dict)} != set(expected_attributes)
            or "minecraft:player" not in suppliers
            or any(name not in entity_facts for name in suppliers)
        ):
            raise ValueError("1.21.1 attribute catalog does not match the vanilla registries")
        shutil.copyfile(attributes, stage / "attributes.json")

        if movement_properties is None or not movement_properties.is_file():
            raise ValueError("official 1.21.1 movement properties are required for runtime resources")
        if movement_properties.stat().st_size < 16:
            raise ValueError("1.21.1 movement properties table is incomplete")
        shutil.copyfile(movement_properties, stage / "movement-properties.nbt")

        if heightmap_predicates is None:
            raise ValueError("exact 1.21.1 heightmap predicates are required for runtime resources")
        motion = heightmap_predicates.get("motion")
        no_leaves = heightmap_predicates.get("no_leaves")
        if (
            heightmap_predicates.get("data_version") != DATA_VERSION
            or heightmap_predicates.get("states") != len(block_facts["states"])
            or not isinstance(motion, list)
            or not isinstance(no_leaves, list)
            or len(motion) != len(block_facts["states"])
            or len(no_leaves) != len(block_facts["states"])
            or any(type(value) is not bool for value in motion)
            or any(type(value) is not bool for value in no_leaves)
        ):
            raise ValueError("1.21.1 heightmap predicates do not match the block-state registry")
        write_json(stage / "heightmap-predicates.json", heightmap_predicates)
        write_json(stage / "facts.json", block_facts)

        model_states = generate_1_21_1_model_states.build(reports / "blocks.json")
        if set(model_states) != set(block_facts["defaults"]):
            raise ValueError("model-state schemas and block facts cover different block sets")
        write_json(stage / "model-states.json", model_states)

        item_count = install_items(reports, stage)
        biome_count = install_biome_parameters(reports, stage)

        manifest = {
            "schema": 1,
            "minecraft_version": VERSION,
            "data_version": DATA_VERSION,
            "protocol_version": PROTOCOL_VERSION,
            "resource_pack_format": 34,
            "data_pack_format": 48,
            "generator": GENERATOR,
            "client_jar_sha1": digest(client_jar, "sha1"),
            "server_game_jar_sha1": digest(server_game_jar, "sha1"),
            "source_report_sha256": {
                name: digest(reports / name, "sha256")
                for name in ("registries.json", "commands.json", "blocks.json")
            },
            "builtin_registry_count": len(inventory["registries"]),
            "builtin_registry_entry_count": sum(
                len(entries) for entries in inventory["registries"].values()
            ),
            "block_count": len(block_facts["defaults"]),
            "block_state_count": len(block_facts["states"]),
            "model_schema_count": len(model_states),
            "heightmap_predicate_state_count": heightmap_predicates["states"],
            "gameplay_fact_state_count": len(gameplay_states),
            "block_behaviour_block_count": len(behaviour_blocks),
            "block_behaviour_state_count": len(behaviour_states),
            "entity_type_count": len(entity_facts),
            "collision_table_bytes": cached_collision.stat().st_size,
            "occlusion_table_bytes": occlusion.stat().st_size,
            "attribute_definition_count": len(definitions),
            "attribute_entity_supplier_count": len(suppliers),
            "movement_properties_bytes": movement_properties.stat().st_size,
            "block_tag_count": len(client_tags),
            "item_component_item_count": item_count,
            "biome_parameter_file_count": biome_count,
            "jar_resource_file_count": resource_count,
            "jar_resource_bytes": resource_bytes,
        }
        write_json(stage / "horizon-resources.json", manifest)

        if output.exists():
            shutil.rmtree(output)
        os.replace(stage, output)
        return manifest
    except Exception:
        shutil.rmtree(stage, ignore_errors=True)
        raise


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--client-jar", required=True, type=Path)
    parser.add_argument("--server-game-jar", required=True, type=Path)
    parser.add_argument("--reports", required=True, type=Path)
    parser.add_argument("--heightmap-predicates", required=True, type=Path)
    parser.add_argument("--gameplay-facts", required=True, type=Path)
    parser.add_argument("--block-behaviours", required=True, type=Path)
    parser.add_argument("--entity-types", required=True, type=Path)
    parser.add_argument("--cached-collision", required=True, type=Path)
    parser.add_argument("--occlusion", required=True, type=Path)
    parser.add_argument("--attributes", required=True, type=Path)
    parser.add_argument("--movement-properties", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    heightmap_predicates = json.loads(args.heightmap_predicates.read_text(encoding="utf-8"))
    result = build(
        args.client_jar,
        args.server_game_jar,
        args.reports,
        args.out,
        heightmap_predicates,
        args.gameplay_facts,
        args.block_behaviours,
        args.entity_types,
        args.cached_collision,
        args.occlusion,
        args.attributes,
        args.movement_properties,
    )
    print(json.dumps({
        "runtime_resources": str(args.out.resolve()),
        "minecraft_version": result["minecraft_version"],
        "data_version": result["data_version"],
        "protocol_version": result["protocol_version"],
        "built_in_registries": result["builtin_registry_count"],
        "built_in_registry_entries": result["builtin_registry_entry_count"],
        "block_states": result["block_state_count"],
        "block_tags": result["block_tag_count"],
        "item_component_items": result["item_component_item_count"],
        "biome_parameter_files": result["biome_parameter_file_count"],
        "resource_files": result["jar_resource_file_count"],
        "resource_bytes": result["jar_resource_bytes"],
    }, indent=2))


if __name__ == "__main__":
    main()
