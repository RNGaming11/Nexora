#!/usr/bin/env python3
"""Generate a version-scoped data component ID inventory from vanilla registry reports."""

import argparse
import hashlib
import json
from pathlib import Path


def build(report_path: Path, version: str, data_version: int) -> dict:
    raw = report_path.read_bytes()
    report = json.loads(raw)
    registry = report.get("minecraft:data_component_type")
    if not isinstance(registry, dict) or not isinstance(registry.get("entries"), dict):
        raise SystemExit("registries report has no minecraft:data_component_type entries")

    entries = registry["entries"]
    names: list[str | None] = [None] * len(entries)
    for name, entry in entries.items():
        protocol_id = entry.get("protocol_id")
        if not isinstance(protocol_id, int) or protocol_id < 0 or protocol_id >= len(names):
            raise SystemExit(f"{name}: invalid or non-contiguous protocol_id {protocol_id!r}")
        if names[protocol_id] is not None:
            raise SystemExit(f"duplicate data component protocol_id {protocol_id}")
        names[protocol_id] = name

    missing = [index for index, name in enumerate(names) if name is None]
    if missing:
        raise SystemExit(f"data component registry has missing IDs: {missing}")

    return {
        "schema": 1,
        "minecraft_version": version,
        "data_version": data_version,
        "registry": "minecraft:data_component_type",
        "source": f"Mojang {version} vanilla server data generator --reports",
        "source_report_sha256": hashlib.sha256(raw).hexdigest(),
        "components": names,
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
        f"Wrote {len(inventory['components'])} {args.version} data component IDs "
        f"to {args.out}"
    )


if __name__ == "__main__":
    main()
