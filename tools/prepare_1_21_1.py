#!/usr/bin/env python3
"""Prepare the official, obfuscated Minecraft 1.21.1 artifacts for the Nexora port."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

try:
    from . import class_index, lab, version_port
except ImportError:
    import class_index
    import lab
    import version_port

VERSION = "1.21.1"
MANIFEST_URL = version_port.MANIFEST_URL
EXPECTED = {
    "protocol_version": 767,
    "world_version": 3955,
    "resource_pack": "34.0",
    "data_pack": "48.0",
    "java_version": 21,
}


def mapping_stats(path):
    classes = members = 0
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        if line[0].isspace():
            members += 1
        elif " -> " in line and line.endswith(":"):
            classes += 1
        else:
            raise ValueError(f"Unrecognized ProGuard mapping line in {path}: {line[:120]}")
    if classes < 1000:
        raise ValueError(f"Mapping file looks incomplete: only {classes} class mappings in {path}")
    return {"classes": classes, "members": members}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jobs", type=int, default=None)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()

    directory = version_port.version_dir(VERSION)
    jars = directory / "jars"
    mappings = directory / "mappings"
    reports = directory / "datagen" / "reports"
    jars.mkdir(parents=True, exist_ok=True)
    mappings.mkdir(parents=True, exist_ok=True)

    manifest = json.loads(version_port.fetch_json(MANIFEST_URL))
    entry = next((item for item in manifest["versions"] if item["id"] == VERSION), None)
    if entry is None or entry.get("type") != "release":
        raise SystemExit(f"Mojang's official manifest does not list {VERSION} as a release")

    version_json = directory / "version.json"
    if args.force or not version_json.exists() or lab.digest(version_json, "sha1") != entry["sha1"]:
        version_json.write_bytes(version_port.fetch_json(entry["url"], entry["sha1"]))
    meta = json.loads(version_json.read_text(encoding="utf-8"))
    if meta.get("id") != VERSION:
        raise ValueError(f"Official metadata identifies itself as {meta.get('id')!r}")
    if not {"client_mappings", "server_mappings"} <= meta["downloads"].keys():
        raise ValueError("Expected Mojang's official client and server mappings for this obfuscated release")

    artifacts = {}
    for key in ("client", "server"):
        item = meta["downloads"][key]
        path = jars / f"{key}.jar"
        if args.force and path.exists():
            path.unlink()
        artifacts[key] = lab.download(item["url"], path, item["sha1"])

    for key in ("client_mappings", "server_mappings"):
        item = meta["downloads"][key]
        path = mappings / f"{key}.txt"
        if args.force and path.exists():
            path.unlink()
        artifacts[key] = lab.download(item["url"], path, item["sha1"])

    artifacts["version_json"] = version_json
    artifacts["server_game"] = version_port.unpack_game(
        artifacts["server"], jars / "server-game.jar", VERSION
    )
    facts = version_port.jar_facts(artifacts["client"])
    checks = {
        "id": facts["id"] == VERSION,
        "protocol_version": facts.get("protocol_version") == EXPECTED["protocol_version"],
        "world_version": facts.get("world_version") == EXPECTED["world_version"],
        "resource_pack": facts.get("resource_pack") == EXPECTED["resource_pack"],
        "data_pack": facts.get("data_pack") == EXPECTED["data_pack"],
        "java_version": int(facts.get("java_version", 0)) == EXPECTED["java_version"],
    }
    if not all(checks.values()):
        raise ValueError(f"Embedded client version.json did not match expected 1.21.1 facts: {checks}")

    mapping_report = {
        "client": mapping_stats(artifacts["client_mappings"]),
        "server": mapping_stats(artifacts["server_mappings"]),
    }

    if not reports.exists():
        print("Running the official 1.21.1 server data generator with Java 21", flush=True)
        version_port.run_datagen(artifacts["server"], reports, EXPECTED["java_version"])

    print("Indexing raw obfuscated class names for provenance", flush=True)
    classes = class_index.index_jars(
        [artifacts["client"], artifacts["server_game"]], jobs=args.jobs
    )
    classes_path = directory / "classes-obfuscated.json"
    classes_path.write_text(json.dumps(classes, separators=(",", ":")), encoding="utf-8")

    record = {
        "version": VERSION,
        "source": "Mojang version_manifest_v2.json",
        "version_manifest_entry": entry,
        "facts": facts,
        "fact_checks": checks,
        "mapping_stats": mapping_report,
        "paths": {key: str(path.resolve()) for key, path in artifacts.items()},
        "reports": str(reports.resolve()),
        "obfuscated_class_index": str(classes_path.resolve()),
        "class_count": len(classes),
        "method_count": sum(len(item["methods"]) for item in classes.values()),
        "mapping_applied_to_class_index": False,
        "next_required_step": "Apply official ProGuard mappings before comparing symbols with deobfuscated Rust-side references.",
    }
    lab.write_json(directory / "facts.json", facts)
    lab.write_json(directory / "prepared.json", record)
    print(json.dumps({
        "version": VERSION,
        "facts": facts,
        "mapping_stats": mapping_report,
        "class_count": record["class_count"],
        "method_count": record["method_count"],
        "prepared_directory": str(directory.resolve()),
        "reports": str(reports.resolve()),
        "warning": record["next_required_step"],
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
