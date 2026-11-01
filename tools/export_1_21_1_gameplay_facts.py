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


EXPORTER = "dev.horizonlab.parity.Vanilla1211GameplayFactsExporter"
SOURCE = (
    Path(__file__).resolve().parents[1]
    / "tools/java/dev/horizonlab/parity/Vanilla1211GameplayFactsExporter.java"
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


def run_exporter(
    named_jar: Path,
    server_libraries: list[Path],
    source: Path,
    main_class: str,
    arguments: list[str],
    output_directory: Path,
    label: str,
    heap: str = "1G",
    extra_sources: list[Path] | None = None,
) -> None:
    java, javac = java_tools()
    classpath = os.pathsep.join(
        [str(named_jar.resolve()), *(str(path.resolve()) for path in server_libraries)]
    )
    with tempfile.TemporaryDirectory(
        prefix=f"nexora-{label}-1211-", dir=output_directory
    ) as temp:
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
                *[str(path) for path in [source, *(extra_sources or [])]],
            ],
            capture_output=True,
            text=True,
            timeout=180,
        )
        if compile_result.returncode:
            raise SystemExit(
                f"The 1.21.1-specific {label} exporter does not compile against the "
                "officially remapped 1.21.1 server API:\\n"
                + (compile_result.stdout + compile_result.stderr)[-10000:]
            )

        runtime_classpath = os.pathsep.join([str(classes), classpath])
        result = subprocess.run(
            [str(java), f"-Xmx{heap}", "-cp", runtime_classpath, main_class, *arguments],
            capture_output=True,
            text=True,
            timeout=600,
        )
        if result.returncode:
            raise SystemExit(
                f"The official 1.21.1 {label} export failed:\\n"
                + (result.stdout + result.stderr)[-10000:]
            )


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
        "ignited_by_lava",
        "push",
        "full_collision",
        "friction",
        "explosion_resistance",
        "item",
        "sound",
        "outline",
        "outline_edges",
        "outline_offset",
        "map_color",
        "offset",
    }
    boolean_fields = {
        "skylight_down",
        "shape_occlusion",
        "can_occlude",
        "solid_render",
        "requires_tool",
        "replaceable",
        "random_ticks",
        "block_entity",
        "liquid",
        "ignited_by_lava",
        "full_collision",
        "outline_offset",
        "offset",
    }
    numeric_fields = {
        "emission",
        "dampening",
        "hardness",
        "friction",
        "explosion_resistance",
        "map_color",
    }
    for index, entry in enumerate(states):
        if not isinstance(entry, dict) or entry.get("id") != index:
            raise ValueError(f"Gameplay facts state {index} is malformed or out of order")
        missing = sorted(required - entry.keys())
        if missing:
            raise ValueError(f"Gameplay facts state {index} lacks fields: {missing}")
        if any(type(entry[field]) is not bool for field in boolean_fields):
            raise ValueError(f"Gameplay facts state {index} has invalid boolean fields")
        if any(not isinstance(entry[field], (int, float)) for field in numeric_fields):
            raise ValueError(f"Gameplay facts state {index} has invalid numeric fields")
        for field in ("outline", "outline_edges"):
            geometry = entry[field]
            if not isinstance(geometry, list) or any(
                not isinstance(box, list) or len(box) != 6
                or any(not isinstance(value, (int, float)) for value in box)
                for box in geometry
            ):
                raise ValueError(f"Gameplay facts state {index} has invalid {field}")
        sound = entry["sound"]
        if not isinstance(sound, dict) or not all(
            isinstance(sound.get(key), dict)
            and isinstance(sound[key].get("id"), str)
            for key in ("break", "step", "place", "hit", "fall")
        ):
            raise ValueError(f"Gameplay facts state {index} has invalid sound IDs")
        if not isinstance(sound.get("volume"), (int, float)) or not isinstance(
            sound.get("pitch"), (int, float)
        ):
            raise ValueError(f"Gameplay facts state {index} has invalid sound volume or pitch")
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

    run_exporter(
        named_jar,
        server_libraries,
        SOURCE,
        EXPORTER,
        [str(output)],
        output.parent,
        "gameplay-facts",
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
