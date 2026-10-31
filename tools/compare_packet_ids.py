#!/usr/bin/env python3
"""Compare generated packet-ID Rust modules across two Minecraft versions."""

import argparse
import json
import re
from pathlib import Path


MODULE = re.compile(r"^\s*pub mod ([a-z_]+) \{$")
CONST = re.compile(r"^\s*pub const ([A-Z0-9_]+): i32 = (-?\d+);$")


def read_ids(path):
    result = {}
    state = direction = None
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        match = MODULE.match(line)
        if match:
            name = match.group(1)
            if state is None:
                state = name
            elif direction is None:
                direction = name
            continue
        if line.strip() == "}":
            if direction is not None:
                direction = None
            elif state is not None:
                state = None
            continue
        match = CONST.match(line)
        if match and state and direction:
            result[(state, direction, match.group(1))] = int(match.group(2))
    return result


def compare(current, target):
    keys = current.keys() | target.keys()
    added, removed, changed = [], [], []
    for key in sorted(keys):
        old, new = current.get(key), target.get(key)
        item = {"state": key[0], "direction": key[1], "packet": key[2]}
        if old is None:
            added.append({**item, "target_id": new})
        elif new is None:
            removed.append({**item, "current_id": old})
        elif old != new:
            changed.append({**item, "current_id": old, "target_id": new})
    return {
        "current_packet_count": len(current),
        "target_packet_count": len(target),
        "added": added,
        "removed": removed,
        "renumbered": changed,
        "summary": {
            "added": len(added),
            "removed": len(removed),
            "renumbered": len(changed),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--current", type=Path, default=Path("crates/horizon-protocol/src/ids.rs"))
    parser.add_argument("--target", type=Path, default=Path("ports/1.21.1/packet_ids.rs"))
    parser.add_argument("--out", type=Path, default=Path("ports/1.21.1/packet-id-diff.json"))
    args = parser.parse_args()
    result = compare(read_ids(args.current), read_ids(args.target))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result["summary"], sort_keys=True))
    print(f"Detailed packet comparison: {args.out}")


if __name__ == "__main__":
    main()
