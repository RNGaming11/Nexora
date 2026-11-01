#!/usr/bin/env python3
"""Remap an obfuscated Minecraft 1.21.1 jar using Mojang's official mappings."""

import os
from pathlib import Path
import subprocess
import tempfile

try:
    from . import lab, proguard, version_port
except ImportError:
    import lab
    import proguard
    import version_port


JVM_EXPORTS = [
    "--add-exports=java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED",
    "--add-exports=java.base/jdk.internal.org.objectweb.asm.commons=ALL-UNNAMED",
]


def write_mapping_table(mapping: proguard.ProguardMappings, path: Path) -> None:
    rows = []
    for obfuscated, source in mapping.obfuscated_to_source.items():
        rows.append(("C", obfuscated, source))
    for (owner, name, descriptor), source in mapping.fields.items():
        rows.append(("F", owner, name, descriptor, source))
    for (owner, name, descriptor), source in mapping.methods.items():
        rows.append(("M", owner, name, descriptor, source))
    with path.open("w", encoding="utf-8", newline="\n") as stream:
        for row in rows:
            if any("\t" in part or "\n" in part for part in row):
                raise ValueError("A ProGuard mapping member contains a tab or newline")
            stream.write("\t".join(row) + "\n")


def remap(
    input_jar: Path,
    mappings_path: Path,
    libraries: list[Path],
    output_jar: Path,
) -> dict:
    input_jar = input_jar.resolve()
    mappings_path = mappings_path.resolve()
    output_jar = output_jar.resolve()
    if not input_jar.is_file() or not mappings_path.is_file():
        raise ValueError("The verified jar and its official ProGuard mapping file are required")
    missing = [str(path) for path in libraries if not path.is_file()]
    if missing:
        raise ValueError(f"Bundled Minecraft libraries are missing: {missing[:4]}")
    if input_jar == output_jar:
        raise ValueError("Refusing to overwrite the verified source jar")

    mapping = proguard.ProguardMappings.read(mappings_path)
    output_jar.parent.mkdir(parents=True, exist_ok=True)
    java = Path(version_port.java_command(21)).resolve()
    java_home = Path(os.environ.get("JAVA_HOME", java.parent))
    javac = java_home / "bin" / ("javac.exe" if os.name == "nt" else "javac")
    if not javac.is_file():
        javac = java.with_name("javac.exe" if os.name == "nt" else "javac")
    if not javac.is_file():
        raise SystemExit(f"JDK compiler not found next to {java}")

    source = Path(__file__).with_name("java") / "ProguardJarRemapper.java"
    with tempfile.TemporaryDirectory(prefix="nexora-remap-", dir=output_jar.parent) as temp:
        work = Path(temp)
        mapping_table = work / "mappings.tsv"
        classes = work / "classes"
        classes.mkdir()
        write_mapping_table(mapping, mapping_table)
        compile_result = subprocess.run(
            [str(javac), *JVM_EXPORTS, "-d", str(classes), str(source)],
            capture_output=True,
            text=True,
            timeout=120,
        )
        if compile_result.returncode:
            raise SystemExit(
                "Could not compile the official ProGuard jar remapper:\n"
                + (compile_result.stdout + compile_result.stderr)[-6000:]
            )

        runtime_classpath = str(classes)
        result = subprocess.run(
            [
                str(java),
                *JVM_EXPORTS,
                "-cp",
                runtime_classpath,
                "ProguardJarRemapper",
                str(input_jar),
                str(mapping_table),
                str(output_jar),
            ],
            capture_output=True,
            text=True,
            timeout=900,
        )
        if result.returncode:
            output_jar.unlink(missing_ok=True)
            raise SystemExit(
                "Official ProGuard jar remapping failed:\n"
                + (result.stdout + result.stderr)[-8000:]
            )
        print(result.stdout.strip())

    return {
        "input_sha256": lab.digest(input_jar, "sha256"),
        "output_sha256": lab.digest(output_jar, "sha256"),
        "class_mapping_count": len(mapping.obfuscated_to_source),
        "method_mapping_count": len(mapping.methods),
        "field_mapping_count": len(mapping.fields),
        "output": str(output_jar),
    }


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-jar", required=True, type=Path)
    parser.add_argument("--mappings", required=True, type=Path)
    parser.add_argument("--library", action="append", default=[], type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    print(remap(args.input_jar, args.mappings, args.library, args.out))


if __name__ == "__main__":
    main()
