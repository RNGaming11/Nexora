#!/usr/bin/env python3
"""Export 1.21.1 block behaviour tables using the existing vanilla exporter backported to its API."""

import json
from pathlib import Path

try:
    from . import export_1_21_1_gameplay_facts, generate_1_21_1_block_facts
except ImportError:
    import export_1_21_1_gameplay_facts
    import generate_1_21_1_block_facts


EXPORTER = "dev.horizonlab.parity.Vanilla1211BlockBehaviourExporter"
SOURCE = (
    Path(__file__).resolve().parents[1]
    / "tools/java/dev/horizonlab/parity/Vanilla1211BlockBehaviourExporter.java"
)


def validate(path: Path, block_report: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("1.21.1 block behaviour export must be a JSON object")

    expected = generate_1_21_1_block_facts.build(block_report, 3955)
    expected_blocks = len(expected["defaults"])
    expected_states = len(expected["states"])
    blocks = value.get("blocks")
    states = value.get("states")
    items = value.get("items")
    fluids = value.get("fluids")
    fire = value.get("fire")

    if not isinstance(blocks, list) or len(blocks) != expected_blocks:
        raise ValueError(
            f"1.21.1 block behaviour export has {len(blocks) if isinstance(blocks, list) else 'no'} "
            f"blocks; expected {expected_blocks}"
        )
    if not isinstance(states, list) or len(states) != expected_states:
        raise ValueError(
            f"1.21.1 block behaviour export has {len(states) if isinstance(states, list) else 'no'} "
            f"states; expected {expected_states}"
        )
    if not isinstance(items, dict) or not isinstance(fluids, list) or not isinstance(fire, dict):
        raise ValueError("1.21.1 block behaviour export is missing items, fluids or fire data")

    for index, block in enumerate(blocks):
        if (
            not isinstance(block, dict)
            or not isinstance(block.get("name"), str)
            or not isinstance(block.get("class"), list)
            or not isinstance(block.get("interfaces"), list)
            or not isinstance(block.get("fields"), dict)
            or type(block.get("dynamic_shape")) is not bool
            or not isinstance(block.get("item"), str)
        ):
            raise ValueError(f"1.21.1 block behaviour entry {index} is malformed")

    for index, state in enumerate(states):
        if not isinstance(state, list) or len(state) != 9:
            raise ValueError(f"1.21.1 block behaviour state {index} is malformed")
        if (
            any(type(state[i]) is not int for i in (0, 1, 2, 3))
            or type(state[4]) is not bool
            or any(type(state[i]) is not int for i in (5, 6))
            or type(state[7]) is not bool
            or not isinstance(state[8], str)
        ):
            raise ValueError(f"1.21.1 block behaviour state {index} has invalid values")

    if any(not isinstance(name, str) for name in fluids):
        raise ValueError("1.21.1 block behaviour fluid IDs must be strings")

    return {
        "blocks": len(blocks),
        "states": len(states),
        "items": len(items),
        "fluids": len(fluids),
        "bytes": path.stat().st_size,
    }


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
        raise ValueError("The mapped official server jar and 1.21.1 blocks report are required")
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
        "block-behaviours",
    )
    summary = validate(output, block_report)
    print(
        "Exported exact Minecraft 1.21.1 block behaviour tables: "
        f"{summary['blocks']} blocks, {summary['states']} states"
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
