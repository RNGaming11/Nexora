#!/usr/bin/env python3
"""Export target-version per-state gameplay facts using Mojang's mapped 1.21.1 server."""

import json
import os
from pathlib import Path
import subprocess
import tempfile

try:
    from . import generate_1_21_1_block_facts, remap_proguard_jar, version_port
except ImportError:
    import generate_1_21_1_block_facts
    import remap_proguard_jar
    import version_port


EXPORTER = "dev.horizonlab.parity.VanillaGameplayFactsExporter"
SOURCE = (
    Path(__file__).resolve().parents[1]
    / "mods/horizon/src/test/java/dev/horizonlab/parity/VanillaGameplayFactsExporter.java"
)


def java_tools() -> tuple[Path, Path]:
    java = Path(version_port.java_command(21)).resolve()
    home = Path(os.environ.get("JAVA_HOME", java.parent))
    javac = home / "bin" / ("javac.exe" if os.name == "nt" else "javac")
    if not javac.is_file():
        javac = java.with_name("javac.exe" if os.name == "nt" else "javac")
    if not javac.is_file():
        raise SystemExit(f"JDK compiler not found next to {java}")
    return java, javac


def validate(path: Path, block_report: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    states = value.get("states") if isinstance(value, dict) else None
    if not isinstance(states, list) or not states:
        raise ValueError("1.21.1 gameplay facts contain no block states")
    expected = generate_1_21_1_block_facts.build(block_report, 3955)
    if len(states) != len(expected["states"]):
        raise ValueError(
            f"1.21.1 gameplay facts cover {len(states)} states, "
            f"but the official registry has {len(expected['states'])}"
        )
    required = {
        "emission",
        "dampening",
        "skylight_down",
        "shape_occlusion",
        "can_occlude",
        "solid_render",
        "hardness",
        "requires_tool",
        "replaceable",
        "random_ticks",
        "block_entity",
        "render_shape",
        "liquid",
        "push",
        "full_collision",
        "friction",
        "explosion_resistance",
        "item",
        "sound",
        "outline",
        "outline_edges",
    }
    for index, entry in enumerate(states):
        if not isinstance(entry, dict) or entry.get("id") != index:
            raise ValueError(f"Gameplay facts state {index} is malformed or out of order")
        missing = sorted(required - entry.keys())
        if missing:
            raise ValueError(f"Gameplay facts state {index} lacks fields: {missing}")
        for field in ("outline", "outline_edges"):
            if not isinstance(entry[field], list):
                raise ValueError(f"Gameplay facts state {index} has invalid {field}")
    return {"states": len(states), "bytes": path.stat().st_size}


def export(
    server_game_jar: Path,
    server_mappings: Path,
    server_libraries: list[Path],
    block_report: Path,
    output: Path,
) -> dict:
    server_game_jar = server_game_jar.resolve()
    server_mappings = server_mappings.resolve()
    block_report = block_report.resolve()
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    named_jar = output.parent / "server-game-named.jar"
    remap_proguard_jar.remap(
        server_game_jar,
        server_mappings,
        server_libraries,
        named_jar,
    )

    java, javac = java_tools()
    classpath = os.pathsep.join(
        [str(named_jar), *(str(path.resolve()) for path in server_libraries)]
    )
    with tempfile.TemporaryDirectory(prefix="nexora-gameplay-1211-", dir=output.parent) as temp:
        classes = Path(temp) / "classes"
        classes.mkdir()
        compile_result = subprocess.run(
            [
                str(javac),
                "--release",
                "21",
                "-proc:none",
                "-cp",
                classpath,
                "-d",
                str(classes),
                str(SOURCE),
            ],
            capture_output=True,
            text=True,
            timeout=180,
        )
        if compile_result.returncode:
            raise SystemExit(
                "The existing vanilla gameplay exporter does not compile against the "
                "officially remapped 1.21.1 server API:\n"
                + (compile_result.stdout + compile_result.stderr)[-10000:]
            )

        runtime_classpath = os.pathsep.join([str(classes), classpath])
        result = subprocess.run(
            [str(java), "-Xmx1G", "-cp", runtime_classpath, EXPORTER, str(output)],
            capture_output=True,
            text=True,
            timeout=300,
        )
        if result.returncode:
            output.unlink(missing_ok=True)
            raise SystemExit(
                "The official 1.21.1 gameplay fact export failed:\n"
                + (result.stdout + result.stderr)[-10000:]
            )

    summary = validate(output, block_report)
    summary["named_server_jar"] = str(named_jar)
    print(
        f"Exported exact Minecraft 1.21.1 gameplay facts: "
        f"{summary['states']} states, {summary['bytes']:,} bytes"
    )
    return summary


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-game-jar", required=True, type=Path)
    parser.add_argument("--server-mappings", required=True, type=Path)
    parser.add_argument("--library", action="append", default=[], type=Path)
    parser.add_argument("--blocks-report", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    print(
        json.dumps(
            export(
                args.server_game_jar,
                args.server_mappings,
                args.library,
                args.blocks_report,
                args.out,
            ),
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
