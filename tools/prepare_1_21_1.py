#!/usr/bin/env python3
"""Prepare the official, obfuscated Minecraft 1.21.1 artifacts for the Nexora port."""

import argparse
import hashlib
import json
from pathlib import Path
import sys
import zipfile

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
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        if not line or line.lstrip().startswith("#"):
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



def mapping_coverage(jar, mapping_path):
    try:
        from .proguard import ProguardMappings
    except ImportError:
        from proguard import ProguardMappings
    mapping = ProguardMappings.read(mapping_path)
    with zipfile.ZipFile(jar) as archive:
        jar_classes = {name[:-6] for name in archive.namelist() if name.endswith(".class")}
    mapped_names = set(mapping.obfuscated_to_source)
    present = mapped_names & jar_classes
    absent = mapped_names - jar_classes
    return {
        "mapping_classes": len(mapped_names),
        "jar_classes": len(jar_classes),
        "matched_classes": len(present),
        "mapping_classes_missing_from_jar": len(absent),
        "mapped_class_names": {mapping.obfuscated_to_source[name] for name in present},
    }

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
        "java_version": (facts.get("java_version", {}).get("major") if isinstance(facts.get("java_version"), dict) else facts.get("java_version")) == EXPECTED["java_version"],
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

    client_coverage = mapping_coverage(artifacts["client"], artifacts["client_mappings"])
    server_coverage = mapping_coverage(artifacts["server_game"], artifacts["server_mappings"])
    client_names = client_coverage.pop("mapped_class_names")
    server_names = server_coverage.pop("mapped_class_names")
    coverage_report = {
        "client": client_coverage,
        "server": server_coverage,
        "overlap": len(client_names & server_names),
        "client_only": len(client_names - server_names),
        "server_only": len(server_names - client_names),
        "union": len(client_names | server_names),
    }
    print("Mapping coverage: " + json.dumps(coverage_report, sort_keys=True), flush=True)

    print("Applying official client ProGuard mappings and indexing client classes", flush=True)
    client_classes = class_index.index_jars(
        [artifacts["client"]],
        jobs=args.jobs,
        mappings=[artifacts["client_mappings"]],
    )
    client_classes_path = directory / "classes-client-mapped.json"
    client_classes_path.write_text(
        json.dumps(client_classes, separators=(",", ":")), encoding="utf-8"
    )

    print("Applying official server ProGuard mappings and indexing server classes", flush=True)
    server_classes = class_index.index_jars(
        [artifacts["server_game"]],
        jobs=args.jobs,
        mappings=[artifacts["server_mappings"]],
    )
    server_classes_path = directory / "classes-server-mapped.json"
    server_classes_path.write_text(
        json.dumps(server_classes, separators=(",", ":")), encoding="utf-8"
    )

    print("Comparing mapped client classes against mapped server classes", flush=True)
    client_server_diff = class_index.diff(client_classes, server_classes)
    client_server_diff_path = directory / "classes-client-server-diff.json"
    client_server_diff_path.write_text(
        json.dumps(client_server_diff, separators=(",", ":")), encoding="utf-8"
    )
    client_server_summary = {
        "server_only_classes": len(client_server_diff["added"]),
        "client_only_classes": len(client_server_diff["removed"]),
        "shared_classes": len(client_classes.keys() & server_classes.keys()),
        "moved_classes": len(client_server_diff["moved"]),
        "changed_common_classes": len(client_server_diff["changed"]),
        "changed_methods": sum(
            class_index.changed_methods(change)
            for change in client_server_diff["changed"].values()
        ),
        "diff_file": str(client_server_diff_path.resolve()),
    }
    print("Client/server implementation comparison: " +
          json.dumps(client_server_summary, sort_keys=True), flush=True)

    mapped_classes = dict(client_classes)
    for name, server_data in server_classes.items():
        client_data = mapped_classes.get(name)
        if client_data is None:
            mapped_classes[name] = server_data
            continue
        merged = dict(client_data)
        merged["methods"] = {**client_data["methods"], **server_data["methods"]}
        merged["fields"] = {**client_data["fields"], **server_data["fields"]}
        mapped_classes[name] = merged
    mapped_classes_path = directory / "classes-mapped.json"
    mapped_classes_path.write_text(
        json.dumps(mapped_classes, separators=(",", ":")), encoding="utf-8"
    )
    # Keep the standard version-port diff pipeline usable: it expects the
    # version's canonical class inventory at classes.json. For 1.21.1 that
    # inventory must be the official-mapping-normalized union, never raw names.
    classes_path = directory / "classes.json"
    classes_path.write_text(
        json.dumps(mapped_classes, separators=(",", ":")), encoding="utf-8"
    )

    record = {
        "version": VERSION,
        "source": "Mojang version_manifest_v2.json",
        "version_manifest_entry": entry,
        "facts": facts,
        "fact_checks": checks,
        "mapping_stats": mapping_report,
        "mapping_coverage": coverage_report,
        "paths": {key: str(path.resolve()) for key, path in artifacts.items()},
        "reports": str(reports.resolve()),
        "obfuscated_class_index": str(classes_path.resolve()),
        "class_count": len(classes),
        "method_count": sum(len(item["methods"]) for item in classes.values()),
        "mapped_class_index": str(mapped_classes_path.resolve()),
        "client_server_diff": client_server_summary,
        "client_class_index": str(client_classes_path.resolve()),
        "server_class_index": str(server_classes_path.resolve()),
        "client_mapped_class_count": len(client_classes),
        "client_mapped_method_count": sum(len(item["methods"]) for item in client_classes.values()),
        "server_mapped_class_count": len(server_classes),
        "server_mapped_method_count": sum(len(item["methods"]) for item in server_classes.values()),
        "mapped_class_count": len(mapped_classes),
        "mapped_method_count": sum(len(item["methods"]) for item in mapped_classes.values()),
        "mapping_applied_to_class_index": True,
        "next_required_step": "Compare the mapped inventory against the 1.21.1-specific candidate baseline; raw obfuscated inventory is retained for provenance.",
    }
    lab.write_json(directory / "facts.json", facts)
    lab.write_json(directory / "prepared.json", record)
    print(json.dumps({
        "version": VERSION,
        "facts": facts,
        "mapping_stats": mapping_report,
        "mapping_coverage": coverage_report,
        "class_count": record["class_count"],
        "method_count": record["method_count"],
        "mapped_class_count": record["mapped_class_count"],
        "mapped_method_count": record["mapped_method_count"],
        "prepared_directory": str(directory.resolve()),
        "reports": str(reports.resolve()),
        "mapped_class_index": str(mapped_classes_path.resolve()),
        "client_server_diff": client_server_summary,
        "client_class_index": str(client_classes_path.resolve()),
        "server_class_index": str(server_classes_path.resolve()),
        "client_mapped_class_count": record["client_mapped_class_count"],
        "client_mapped_method_count": record["client_mapped_method_count"],
        "server_mapped_class_count": record["server_mapped_class_count"],
        "server_mapped_method_count": record["server_mapped_method_count"],
        "next_step": record["next_required_step"],
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
