#!/usr/bin/env python3
"""Download only the exact Mojang-published files pinned by research-lock.json for CI preparation.

The downloaded jars stay on the ephemeral runner. They are never included in release artifacts.
"""
from __future__ import annotations

import hashlib
import json
import os
import urllib.request
from pathlib import Path
from urllib.parse import urlparse


ROOT = Path(__file__).resolve().parents[2]


def sha1(path: Path) -> str:
    digest = hashlib.sha1()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, destination: Path, expected: str) -> None:
    if urlparse(url).scheme != "https":
        raise SystemExit(f"Refusing non-HTTPS Mojang download: {url}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.is_file() and sha1(destination) == expected:
        return
    temporary = destination.with_suffix(destination.suffix + ".download")
    digest = hashlib.sha1()
    request = urllib.request.Request(url, headers={"User-Agent": "Nexora-CI/1.0"})
    try:
        with urllib.request.urlopen(request, timeout=60) as response, temporary.open("wb") as output:
            while chunk := response.read(1024 * 1024):
                digest.update(chunk)
                output.write(chunk)
        actual = digest.hexdigest()
        if actual != expected:
            raise SystemExit(f"SHA-1 mismatch for {destination}: expected {expected}, got {actual}")
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)


def main() -> None:
    lock = json.loads((ROOT / "research-lock.json").read_text(encoding="utf-8"))["minecraft"]
    version = lock["version"]
    minecraft = Path(os.environ.get("RUNNER_TEMP", "/tmp")) / "nexora-minecraft"
    version_dir = minecraft / "versions" / version
    manifest_url = f"https://piston-meta.mojang.com/v1/packages/{lock['manifest_sha1']}/{version}.json"
    manifest_path = version_dir / f"{version}.json"
    download(manifest_url, manifest_path, lock["manifest_sha1"])
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("id") != version:
        raise SystemExit(f"Mojang manifest id mismatch: expected {version}, got {manifest.get('id')}")
    downloads = manifest.get("downloads", {})
    for side in ("client", "server"):
        entry = downloads.get(side, {})
        expected = lock[f"{side}_sha1"]
        if entry.get("sha1") != expected:
            raise SystemExit(f"Pinned {side} SHA-1 does not match Mojang version metadata")
    download(downloads["client"]["url"], version_dir / f"{version}.jar", lock["client_sha1"])
    server_jar = minecraft / "server.jar"
    download(downloads["server"]["url"], server_jar, lock["server_sha1"])

    libraries = manifest.get("libraries", [])
    count = 0
    for library in libraries:
        artifact = library.get("downloads", {}).get("artifact")
        if not artifact:
            continue
        relative = Path(artifact["path"])
        if relative.is_absolute() or ".." in relative.parts or "\\" in artifact["path"]:
            raise SystemExit(f"Unsafe library path in Mojang metadata: {artifact['path']}")
        download(artifact["url"], minecraft / "libraries" / relative, artifact["sha1"])
        count += 1

        # Mojang publishes platform-native jars as classifiers, not artifacts.
        # The preparer needs these too (for example LWJGL's natives-windows jar).
        for classifier in library.get("downloads", {}).get("classifiers", {}).values():
            native_relative = Path(classifier["path"])
            if native_relative.is_absolute() or ".." in native_relative.parts or "\\" in classifier["path"]:
                raise SystemExit(f"Unsafe native library path in Mojang metadata: {classifier['path']}")
            download(
                classifier["url"],
                minecraft / "libraries" / native_relative,
                classifier["sha1"],
            )
            count += 1

    asset_index = manifest.get("assetIndex", {})
    index_path = minecraft / "assets" / "indexes" / f"{asset_index.get('id', '')}.json"
    download(asset_index["url"], index_path, asset_index["sha1"])
    index = json.loads(index_path.read_text(encoding="utf-8"))
    wanted = (
        lambda name: name == "pack.mcmeta"
        or name.startswith("minecraft/lang/")
        or name.startswith("minecraft/font/")
        or name.startswith("minecraft/resourcepacks/")
        or name.startswith("minecraft/textures/gui/title/background/panorama_")
    )
    asset_count = 0
    for name, entry in index.get("objects", {}).items():
        if not wanted(name):
            continue
        checksum = entry["hash"]
        destination = minecraft / "assets" / "objects" / checksum[:2] / checksum
        download(f"https://resources.download.minecraft.net/{checksum[:2]}/{checksum}", destination, checksum)
        asset_count += 1

    print(
        f"Prepared pinned Minecraft {version}: client/server jars, {count} library artifacts, "
        f"and {asset_count} required asset objects."
    )
    print("All Mojang files were SHA-1 checked; none are copied into release packages.")


if __name__ == "__main__":
    main()
