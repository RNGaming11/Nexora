#!/usr/bin/env python3
"""Export entity dimensions, categories and attributes with the 1.21.1 vanilla exporter."""

import json
from pathlib import Path

try:
    from . import export_1_21_1_gameplay_facts, generate_builtin_registry_inventory
except ImportError:
    import export_1_21_1_gameplay_facts
    import generate_builtin_registry_inventory


EXPORTER = "dev.horizonlab.parity.Vanilla1211EntityTypesExporter"
SOURCE = (
    Path(__file__).resolve().parents[1]
    / "tools/java/dev/horizonlab/parity/Vanilla1211EntityTypesExporter.java"
)


def validate(path: Path, registries_report: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict) or not value:
        raise ValueError("1.21.1 entity type export must be a non-empty object")
    inventory = generate_builtin_registry_inventory.build(
        registries_report, "1.21.1", 3955
    )
    expected = inventory["registries"].get("minecraft:entity_type")
    if not isinstance(expected, list) or set(value) != set(expected):
        raise ValueError("1.21.1 entity type facts do not match the vanilla entity registry")
    for name, entry in value.items():
        if not isinstance(entry, dict):
            raise ValueError(f"1.21.1 entity type {name} is malformed")
        for field in ("width", "height", "eye_height"):
            number = entry.get(field)
            if not isinstance(number, (int, float)) or not 0 <= number < 128:
                raise ValueError(f"1.21.1 entity type {name} has invalid {field}")
        if not isinstance(entry.get("category"), str) or type(entry.get("fire_immune")) is not bool:
            raise ValueError(f"1.21.1 entity type {name} has invalid category or fire immunity")
        attributes = entry.get("attributes", {})
        if not isinstance(attributes, dict) or any(
            not isinstance(key, str) or not isinstance(value, (int, float))
            for key, value in attributes.items()
        ):
            raise ValueError(f"1.21.1 entity type {name} has invalid default attributes")
    return {"entity_types": len(value), "bytes": path.stat().st_size}


def export(named_server_jar: Path, server_libraries: list[Path], registries_report: Path, output: Path) -> dict:
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
        [str(output)],
        output.parent,
        "entity-types",
    )
    summary = validate(output, registries_report)
    print(f"Exported exact Minecraft 1.21.1 entity types: {summary['entity_types']}")
    return summary


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
