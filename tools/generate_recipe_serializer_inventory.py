#!/usr/bin/env python3
"""Generate the version-scoped recipe serializer ID inventory from vanilla reports."""

import argparse
import hashlib
import json
from pathlib import Path


def build(report_path: Path, version: str, data_version: int) -> dict:
    raw = report_path.read_bytes()
    report = json.loads(raw)
    registry = report.get("minecraft:recipe_serializer")
    if not isinstance(registry, dict) or not isinstance(registry.get("entries"), dict):
        raise SystemExit("registries report has no minecraft:recipe_serializer entries")

    entries = registry["entries"]
    names: list[str | None] = [None] * len(entries)
    for name, entry in entries.items():
        protocol_id = entry.get("protocol_id")
        if not isinstance(protocol_id, int) or protocol_id < 0 or protocol_id >= len(names):
            raise SystemExit(f"{name}: invalid or non-contiguous protocol_id {protocol_id!r}")
        if names[protocol_id] is not None:
            raise SystemExit(f"duplicate recipe serializer protocol_id {protocol_id}")
        names[protocol_id] = name

    missing = [index for index, name in enumerate(names) if name is None]
    if missing:
        raise SystemExit(f"recipe serializer registry has missing IDs: {missing}")

    return {
        "schema": 1,
        "minecraft_version": version,
        "data_version": data_version,
        "registry": "minecraft:recipe_serializer",
        "source": f"Mojang {version} vanilla server data generator --reports",
        "source_report_sha256": hashlib.sha256(raw).hexdigest(),
        "serializers": names,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--data-version", required=True, type=int)
    args = parser.parse_args()

    inventory = build(args.report, args.version, args.data_version)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8")
    print(
        f"Wrote {len(inventory['serializers'])} {args.version} recipe serializer IDs "
        f"to {args.out}"
    )


if __name__ == "__main__":
    main()
