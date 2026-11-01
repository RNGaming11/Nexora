#!/usr/bin/env python3
"""Generate compact version-specific built-in registry IDs from Mojang reports."""

import argparse
import hashlib
import json
from pathlib import Path


def build(report_path: Path, version: str, data_version: int) -> dict:
    raw = report_path.read_bytes()
    report = json.loads(raw)
    if not isinstance(report, dict):
        raise SystemExit("registries report must be an object")

    registries: dict[str, list[str]] = {}
    for name, registry in sorted(report.items()):
        entries = registry.get("entries") if isinstance(registry, dict) else None
        if not isinstance(entries, dict):
            continue
        names: list[str | None] = [None] * len(entries)
        for entry_name, entry in entries.items():
            protocol_id = entry.get("protocol_id") if isinstance(entry, dict) else None
            if not isinstance(protocol_id, int) or protocol_id < 0 or protocol_id >= len(names):
                raise SystemExit(f"{name}/{entry_name}: invalid or non-contiguous protocol_id {protocol_id!r}")
            if names[protocol_id] is not None:
                raise SystemExit(f"{name}: duplicate protocol_id {protocol_id}")
            names[protocol_id] = entry_name
        missing = [index for index, entry_name in enumerate(names) if entry_name is None]
        if missing:
            raise SystemExit(f"{name}: missing protocol IDs: {missing[:16]}")
        registries[name] = [entry_name for entry_name in names if entry_name is not None]

    if not registries:
        raise SystemExit("registries report contains no built-in registry entries")
    return {
        "schema": 1,
        "minecraft_version": version,
        "data_version": data_version,
        "source": f"Mojang {version} vanilla server data generator --reports",
        "source_report_sha256": hashlib.sha256(raw).hexdigest(),
        "registries": registries,
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
    args.out.write_text(json.dumps(inventory, separators=(",", ":"), ensure_ascii=False) + "\n", encoding="utf-8")
    print(
        f"Wrote {len(inventory['registries'])} built-in registries / "
        f"{sum(map(len, inventory['registries'].values()))} IDs to {args.out}"
    )


if __name__ == "__main__":
    main()
