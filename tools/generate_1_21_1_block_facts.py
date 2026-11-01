#!/usr/bin/env python3
"""Convert Mojang's 1.21.1 blocks report into Horizon's state facts format."""

import argparse
import json
from pathlib import Path

AIR_BLOCKS = {
    "minecraft:air",
    "minecraft:cave_air",
    "minecraft:void_air",
}
FLUID_BLOCKS = {
    "minecraft:water",
    "minecraft:lava",
    "minecraft:bubble_column",
    "minecraft:kelp",
    "minecraft:kelp_plant",
    "minecraft:seagrass",
    "minecraft:tall_seagrass",
}


def build(report_path: Path, data_version: int = 3955) -> dict:
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if not isinstance(report, dict) or not report:
        raise SystemExit("Mojang blocks report must be a non-empty object")

    state_rows: list[list] = []
    defaults: dict[str, int] = {}
    seen: set[int] = set()
    for block_name, block in report.items():
        if not isinstance(block_name, str) or not isinstance(block, dict):
            raise SystemExit("invalid block entry in Mojang report")
        states = block.get("states")
        if not isinstance(states, list) or not states:
            raise SystemExit(f"{block_name}: missing block states")
        selected_default = []
        for state in states:
            if not isinstance(state, dict) or not isinstance(state.get("id"), int):
                raise SystemExit(f"{block_name}: malformed state")
            state_id = state["id"]
            if state_id < 0 or state_id in seen:
                raise SystemExit(f"{block_name}: duplicate or negative state id {state_id}")
            seen.add(state_id)
            properties = state.get("properties", {})
            if not isinstance(properties, dict) or any(
                not isinstance(key, str) or not isinstance(value, str)
                for key, value in properties.items()
            ):
                raise SystemExit(f"{block_name}: invalid state properties")
            if properties:
                state_value = {"id": block_name, "properties": properties}
            else:
                state_value = block_name
            air = block_name in AIR_BLOCKS
            fluid = block_name in FLUID_BLOCKS or properties.get("waterlogged") == "true"
            description = f"Block{{{block_name}}}"
            row = [block_name, air, fluid, description, state_value]
            if len(state_rows) <= state_id:
                state_rows.extend([None] * (state_id + 1 - len(state_rows)))
            if state_rows[state_id] is not None:
                raise SystemExit(f"duplicate block state id {state_id}")
            state_rows[state_id] = row
            if state.get("default") is True:
                selected_default.append(state_id)
        if len(selected_default) != 1:
            raise SystemExit(f"{block_name}: expected exactly one default block state")
        defaults[block_name] = selected_default[0]

    missing = [index for index, row in enumerate(state_rows) if row is None]
    if missing:
        raise SystemExit(f"block state IDs are not contiguous; missing {missing[:16]}")
    return {
        "data_version": data_version,
        "states": state_rows,
        "defaults": defaults,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--data-version", type=int, default=3955)
    args = parser.parse_args()
    facts = build(args.report, args.data_version)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        json.dumps(facts, separators=(",", ":"), ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(
        f"Wrote {len(facts['states'])} block states across "
        f"{len(facts['defaults'])} blocks to {args.out}"
    )


if __name__ == "__main__":
    main()
