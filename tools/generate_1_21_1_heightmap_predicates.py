#!/usr/bin/env python3
"""Export exact 1.21.1 heightmap block predicates from Mojang's server jar."""

import json
import os
from pathlib import Path
import subprocess

try:
    from . import proguard, version_port
except ImportError:
    import proguard
    import version_port

SOURCE_CLASSES = {
    "shared_constants": "net.minecraft.SharedConstants",
    "bootstrap": "net.minecraft.server.Bootstrap",
    "block": "net.minecraft.world.level.block.Block",
    "heightmap_types": "net.minecraft.world.level.levelgen.Heightmap$Types",
}


def runtime_class(mapping, source_name: str) -> str:
    source = source_name.replace(".", "/")
    matches = [
        obfuscated
        for obfuscated, mapped in mapping.obfuscated_to_source.items()
        if mapped == source
    ]
    if len(matches) != 1:
        raise ValueError(f"Expected one mapped class for {source_name}, found {matches[:4]}")
    return matches[0].replace("/", ".")


def runtime_member(mapping, owner: str, source_name: str, kind: str, descriptor: str | None = None) -> str:
    owner_name = runtime_class(mapping, owner).replace(".", "/")
    table = mapping.methods if kind == "method" else mapping.fields
    matches = []
    for (mapped_owner, obfuscated_name, mapped_descriptor), name in table.items():
        if mapped_owner != owner_name or name != source_name:
            continue
        if descriptor is not None and mapped_descriptor != descriptor:
            continue
        matches.append(obfuscated_name)
    unique = sorted(set(matches))
    if not unique:
        return source_name
    if len(unique) != 1:
        raise ValueError(
            f"Ambiguous {kind} mapping for {owner}.{source_name}: {unique}"
        )
    return unique[0]


def export(
    server_game_jar: Path,
    server_libraries: list[Path],
    mappings_path: Path,
    output: Path,
) -> dict:
    server_game_jar = server_game_jar.resolve()
    mappings_path = mappings_path.resolve()
    output = output.resolve()
    if not server_game_jar.is_file() or not mappings_path.is_file():
        raise ValueError("Verified 1.21.1 server jar and official mappings are required")
    missing = [str(path) for path in server_libraries if not path.is_file()]
    if missing:
        raise ValueError(f"Bundled 1.21.1 server libraries are missing: {missing[:4]}")

    mapping = proguard.ProguardMappings.read(mappings_path)
    classes = {
        key: runtime_class(mapping, value)
        for key, value in SOURCE_CLASSES.items()
    }
    methods = {
        "try_detect_version": runtime_member(
            mapping,
            SOURCE_CLASSES["shared_constants"],
            "tryDetectVersion",
            "method",
            "()V",
        ),
        "bootstrap": runtime_member(
            mapping,
            SOURCE_CLASSES["bootstrap"],
            "bootStrap",
            "method",
            "()V",
        ),
        "heightmap_key": runtime_member(
            mapping,
            SOURCE_CLASSES["heightmap_types"],
            "getSerializationKey",
            "method",
            "()Ljava/lang/String;",
        ),
        "heightmap_predicate": runtime_member(
            mapping,
            SOURCE_CLASSES["heightmap_types"],
            "isOpaque",
            "method",
            "()Ljava/util/function/Predicate;",
        ),
        "block_states": runtime_member(
            mapping,
            SOURCE_CLASSES["block"],
            "BLOCK_STATE_REGISTRY",
            "field",
        ),
    }

    java = Path(version_port.java_command(21)).resolve()
    if os.environ.get("JAVA_HOME"):
        javac = Path(os.environ["JAVA_HOME"]) / "bin" / (
            "javac.exe" if os.name == "nt" else "javac"
        )
    else:
        javac = java.with_name("javac.exe" if os.name == "nt" else "javac")
    if not javac.is_file():
        raise SystemExit(f"JDK compiler not found: {javac}")

    source = Path(__file__).with_name("java") / "Vanilla1211HeightmapPredicates.java"
    classes_dir = output.parent / "heightmap-exporter-classes"
    classes_dir.mkdir(parents=True, exist_ok=True)
    output.parent.mkdir(parents=True, exist_ok=True)

    compile_result = subprocess.run(
        [str(javac), "-d", str(classes_dir), str(source)],
        capture_output=True,
        text=True,
        timeout=120,
    )
    if compile_result.returncode:
        raise SystemExit(
            "Could not compile the 1.21.1 heightmap exporter:\n"
            + (compile_result.stdout + compile_result.stderr)[-5000:]
        )

    classpath = os.pathsep.join(
        [str(classes_dir), str(server_game_jar), *(str(path.resolve()) for path in server_libraries)]
    )
    args = [
        classes["shared_constants"],
        methods["try_detect_version"],
        classes["bootstrap"],
        methods["bootstrap"],
        classes["block"],
        methods["block_states"],
        classes["heightmap_types"],
        methods["heightmap_key"],
        methods["heightmap_predicate"],
        str(output),
    ]
    result = subprocess.run(
        [str(java), "-cp", classpath, "Vanilla1211HeightmapPredicates", *args],
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode:
        raise SystemExit(
            "The official 1.21.1 heightmap predicate export failed:\n"
            + (result.stdout + result.stderr)[-8000:]
        )

    facts = json.loads(output.read_text(encoding="utf-8"))
    if facts.get("data_version") != 3955:
        raise ValueError("Heightmap predicates are not pinned to data version 3955")
    count = facts.get("states")
    motion = facts.get("motion")
    no_leaves = facts.get("no_leaves")
    if (
        not isinstance(count, int)
        or count <= 0
        or not isinstance(motion, list)
        or not isinstance(no_leaves, list)
        or len(motion) != count
        or len(no_leaves) != count
        or any(type(value) is not bool for value in motion)
        or any(type(value) is not bool for value in no_leaves)
    ):
        raise ValueError("The official 1.21.1 heightmap predicate export is malformed")
    print(result.stdout.strip())
    return facts


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-game-jar", required=True, type=Path)
    parser.add_argument("--server-mappings", required=True, type=Path)
    parser.add_argument("--library", action="append", default=[], type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    export(args.server_game_jar, args.library, args.server_mappings, args.out)


if __name__ == "__main__":
    main()
