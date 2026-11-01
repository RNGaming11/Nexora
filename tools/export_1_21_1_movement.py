#!/usr/bin/env python3
"""Export vanilla 1.21.1 movement coefficients for the existing Rust movement loader."""

import json
from pathlib import Path

try:
    from . import export_1_21_1_gameplay_facts
except ImportError:
    import export_1_21_1_gameplay_facts


EXPORTER = "dev.horizonlab.parity.Vanilla1211MovementExporter"
SOURCE = (
    Path(__file__).resolve().parents[1]
    / "tools/java/dev/horizonlab/parity/Vanilla1211MovementExporter.java"
)


def export(named_server_jar: Path, server_libraries: list[Path], output: Path) -> dict:
    named_server_jar = named_server_jar.resolve()
    output = output.resolve()
    if not named_server_jar.is_file():
        raise ValueError("The mapped official 1.21.1 server jar is required")
    if not all(path.is_file() for path in server_libraries):
        raise ValueError("One or more verified 1.21.1 server libraries are missing")
    output.mkdir(parents=True, exist_ok=True)
    export_1_21_1_gameplay_facts.run_exporter(
        named_server_jar,
        server_libraries,
        SOURCE,
        EXPORTER,
        [str(output)],
        output,
        "movement-properties",
    )
    summary = {}
    for name in ("movement-properties.nbt", "block-registry.nbt"):
        path = output / name
        if not path.is_file() or path.stat().st_size < 16:
            raise ValueError(f"The 1.21.1 movement exporter did not create {name}")
        summary[name] = {"bytes": path.stat().st_size}
    print("Exported exact Minecraft 1.21.1 movement properties")
    return {"files": summary}


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--named-server-jar", required=True, type=Path)
    parser.add_argument("--library", action="append", default=[], type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(export(args.named_server_jar, args.library, args.out), indent=2))


if __name__ == "__main__":
    main()
