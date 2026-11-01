#!/usr/bin/env python3
"""Build model-state property schemas from Mojang's version-specific blocks report."""

import argparse
import json
import re
from pathlib import Path

_INTEGER = re.compile(r"-?[0-9]+\Z")


def build(report_path: Path) -> dict:
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if not isinstance(report, dict) or not report:
        raise SystemExit("Mojang blocks report must be a non-empty object")

    output = {}
    for block_name, block in sorted(report.items()):
        if not isinstance(block_name, str) or not isinstance(block, dict):
            raise SystemExit("invalid block entry in Mojang blocks report")
        states = block.get("states")
        if not isinstance(states, list) or not states:
            raise SystemExit(f"{block_name}: missing block states")

        state_properties = []
        for state in states:
            if not isinstance(state, dict):
                raise SystemExit(f"{block_name}: malformed block state")
            properties = state.get("properties", {})
            if not isinstance(properties, dict) or any(
                not isinstance(key, str) or not isinstance(value, str)
                for key, value in properties.items()
            ):
                raise SystemExit(f"{block_name}: invalid state properties")
            state_properties.append(properties)

        property_names = sorted(state_properties[0])
        if any(sorted(properties) != property_names for properties in state_properties):
            raise SystemExit(f"{block_name}: block states do not share one property schema")

        schema = {}
        for name in property_names:
            values = {properties[name] for properties in state_properties}
            if values and all(_INTEGER.fullmatch(value) for value in values):
                numbers = [int(value) for value in values]
                schema[name] = {"integer": [min(numbers), max(numbers)]}
            else:
                schema[name] = {"named": sorted(values)}

        output[block_name] = {"schema": schema, "states": state_properties}
    return output


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    result = build(args.report)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        json.dumps(result, separators=(",", ":"), ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"Wrote model-state schemas for {len(result)} blocks to {args.out}")


if __name__ == "__main__":
    main()
