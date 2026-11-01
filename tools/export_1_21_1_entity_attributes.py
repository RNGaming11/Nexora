#!/usr/bin/env python3
"""Export the registered 1.21.1 entity attribute catalog with the existing vanilla oracle."""

import json
import re
from pathlib import Path

try:
    from . import export_1_21_1_gameplay_facts, generate_builtin_registry_inventory
except ImportError:
    import export_1_21_1_gameplay_facts
    import generate_builtin_registry_inventory


EXPORTER = "dev.horizonlab.parity.Vanilla1211EntityAttributeExporter"
SOURCE = (
    Path(__file__).resolve().parents[1]
    / "tools/java/dev/horizonlab/parity/Vanilla1211EntityAttributeExporter.java"
)
BITS = re.compile(r"^[0-9a-f]{16}$")


def validate(path: Path, registries_report: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict) or value.get("data_version") != 3955:
        raise ValueError("1.21.1 attribute catalog has the wrong DataVersion")
    inventory = generate_builtin_registry_inventory.build(
        registries_report, "1.21.1", 3955
    )
    entities = inventory["registries"].get("minecraft:entity_type")
    registered_attributes = inventory["registries"].get("minecraft:attribute")
    definitions = value.get("attributes")
    suppliers = value.get("entities")
    if not isinstance(entities, list) or not isinstance(registered_attributes, list):
        raise ValueError("1.21.1 reports are missing entity or attribute registries")
    if not isinstance(definitions, list) or not definitions:
        raise ValueError("1.21.1 attribute catalog has no registered definitions")
    if not isinstance(suppliers, dict) or "minecraft:player" not in suppliers:
        raise ValueError("1.21.1 attribute catalog has no player supplier")

    known = set()
    for index, definition in enumerate(definitions):
        if not isinstance(definition, dict):
            raise ValueError(f"1.21.1 attribute definition {index} is malformed")
        name = definition.get("id")
        if not isinstance(name, str) or name not in registered_attributes or name in known:
            raise ValueError(f"1.21.1 attribute definition {index} has an invalid registry ID")
        known.add(name)
        if not isinstance(definition.get("default"), str) or not BITS.fullmatch(definition["default"]):
            raise ValueError(f"1.21.1 attribute {name} has an invalid default value")
        if type(definition.get("syncable")) is not bool:
            raise ValueError(f"1.21.1 attribute {name} has no sync flag")
        for field in ("min", "max"):
            if field in definition and (
                not isinstance(definition[field], str) or not BITS.fullmatch(definition[field])
            ):
                raise ValueError(f"1.21.1 attribute {name} has an invalid {field} value")
        if ("min" in definition) != ("max" in definition):
            raise ValueError(f"1.21.1 attribute {name} has an incomplete range")

    if known != set(registered_attributes):
        missing = sorted(set(registered_attributes) - known)
        raise ValueError(f"1.21.1 attribute catalog is missing registry entries: {missing[:12]}")

    for entity, bases in suppliers.items():
        if entity not in entities or not isinstance(bases, dict):
            raise ValueError(f"1.21.1 attribute supplier has an invalid entity: {entity}")
        for name, bits in bases.items():
            if name not in known or not isinstance(bits, str) or not BITS.fullmatch(bits):
                raise ValueError(f"1.21.1 entity {entity} has an invalid base for {name}")

    return {
        "attributes": len(definitions),
        "entity_suppliers": len(suppliers),
        "bytes": path.stat().st_size,
    }


def export(
    named_server_jar: Path,
    server_libraries: list[Path],
    registries_report: Path,
    output: Path,
) -> dict:
    named_server_jar = named_server_jar.resolve()
    registries_report = registries_report.resolve()
    output = output.resolve()
    if not named_server_jar.is_file() or not registries_report.is_file():
        raise ValueError("The mapped official server jar and registry report are required")
    if not all(path.is_file() for path in server_libraries):
        raise ValueError("One or more verified 1.21.1 server libraries are missing")
    output.parent.mkdir(parents=True, exist_ok=True)
    export_1_21_1_gameplay_facts.run_exporter(
        named_server_jar,
        server_libraries,
        SOURCE,
        EXPORTER,
        ["export", str(output)],
        output.parent,
        "entity-attributes",
    )
    result = validate(output, registries_report)
    print(
        f"Exported exact Minecraft 1.21.1 attribute catalog: "
        f"{result['attributes']} attributes, {result['entity_suppliers']} entity suppliers"
    )
    return result


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--named-server-jar", required=True, type=Path)
    parser.add_argument("--library", action="append", default=[], type=Path)
    parser.add_argument("--registries-report", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    print(
        json.dumps(
            export(args.named_server_jar, args.library, args.registries_report, args.out),
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
