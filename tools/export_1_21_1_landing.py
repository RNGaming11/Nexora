#!/usr/bin/env python3
"""Export target-version landing rules for the existing shared movement/landing system."""

import json
from pathlib import Path

try:
    from . import export_1_21_1_gameplay_facts, generate_1_21_1_block_facts
except ImportError:
    import export_1_21_1_gameplay_facts
    import generate_1_21_1_block_facts


EXPORTER = "dev.horizonlab.parity.Vanilla1211LandingExporter"
SOURCE = (
    Path(__file__).resolve().parents[1]
    / "tools/java/dev/horizonlab/parity/Vanilla1211LandingExporter.java"
)
FAMILIES = {
    "Block",
    "HayBlock",
    "HoneyBlock",
    "SlimeBlock",
    "PowderSnowBlock",
    "PointedDripstoneBlock",
    "FarmlandBlock",
    "TurtleEggBlock",
}


def validate(path: Path, block_report: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    expected = set(generate_1_21_1_block_facts.build(block_report, 3955)["defaults"])
    if not isinstance(value, dict) or set(value) != expected:
        raise ValueError("1.21.1 landing definitions do not cover the vanilla block registry")
    families = {}
    for name, entry in value.items():
        if not isinstance(entry, dict) or entry.get("fall") not in FAMILIES:
            raise ValueError(f"1.21.1 landing definition has an unsupported family: {name}")
        family = entry["fall"]
        families[family] = families.get(family, 0) + 1
        if type(entry.get("bounce")) is not bool:
            raise ValueError(f"1.21.1 landing definition has no bounce flag: {name}")
        if family == "FarmlandBlock" and not isinstance(entry.get("base"), str):
            raise ValueError(f"1.21.1 farmland landing definition has no base block: {name}")
        if family == "HoneyBlock":
            for field in ("volume", "pitch"):
                bits = entry.get(field)
                if not isinstance(bits, str) or len(bits) != 8:
                    raise ValueError(f"1.21.1 honey landing definition has invalid {field}")
                int(bits, 16)
        if entry["bounce"]:
            raise ValueError("The 1.21.1 landing table unexpectedly contains a later bounce override")
    return {"blocks": len(value), "families": families, "bytes": path.stat().st_size}


def export(
    named_server_jar: Path,
    server_libraries: list[Path],
    block_report: Path,
    output: Path,
) -> dict:
    named_server_jar = named_server_jar.resolve()
    block_report = block_report.resolve()
    output = output.resolve()
    if not named_server_jar.is_file() or not block_report.is_file():
        raise ValueError("The mapped official 1.21.1 server jar and block report are required")
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
        "landing",
    )
    summary = validate(output, block_report)
    print(
        f"Exported exact Minecraft 1.21.1 landing definitions: "
        f"{summary['blocks']} blocks, {summary['families']}"
    )
    return summary


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--named-server-jar", required=True, type=Path)
    parser.add_argument("--library", action="append", default=[], type=Path)
    parser.add_argument("--blocks-report", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    print(
        json.dumps(
            export(
                args.named_server_jar,
                args.library,
                args.blocks_report,
                args.out,
            ),
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
