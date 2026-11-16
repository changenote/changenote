"""Validate the release version and prepare legacy Cargo build versions."""

import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path


def prepare(root: Path, release_commit: str, releases: list) -> str:
    if not re.fullmatch(r"[0-9a-f]{40}", release_commit):
        raise ValueError("Expected a full lowercase release commit SHA")
    actual = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
    ).strip()
    if actual != release_commit:
        raise ValueError("Source checkout must match the verified release commit")
    config = json.loads((root / ".changenote/config.json").read_text())
    source = config["packages"]["changenote"]
    version_file = source["version_file"]
    if source["path"] != "." or version_file not in (
        "Cargo.toml", "crates/changenote/Cargo.toml", "VERSION"
    ):
        raise ValueError("Expected the changenote Cargo manifest or legacy root VERSION")
    manifest_path = root / ("Cargo.toml" if version_file == "VERSION" else version_file)
    lock_path = root / "Cargo.lock"
    manifest = manifest_path.read_text()
    lock = lock_path.read_text()
    package = tomllib.loads(manifest)["package"]
    version = (
        package["version"]
        if version_file != "VERSION"
        else (root / "VERSION").read_text().strip()
    )
    if not isinstance(version, str) or not re.fullmatch(
        r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version
    ):
        raise ValueError("Release source must contain a stable SemVer version")
    if (
        not isinstance(releases, list)
        or len(releases) != 1
        or not isinstance(releases[0], dict)
        or set(releases[0]) != {"package", "version", "tag"}
        or releases[0]["package"] != "changenote"
        or releases[0]["version"] != version
        or not isinstance(releases[0]["tag"], str)
        or not releases[0]["tag"]
    ):
        raise ValueError("Release source must match verified changenote release metadata")

    locked = [p for p in tomllib.loads(lock)["package"] if p["name"] == "changenote"]
    if package["name"] != "changenote" or len(locked) != 1 or "source" in locked[0]:
        raise ValueError("Expected one local changenote Cargo package")
    if package["version"] != locked[0]["version"]:
        raise ValueError("Cargo.toml and Cargo.lock versions must match")

    if version_file != "VERSION":
        return version

    # Legacy VERSION releases need build-only synchronization of the root package.
    manifest, manifest_count = re.subn(
        r'(?m)(^\[package\]\n(?:(?!\[)[^\n]*\n)*?version = ")[^"]+("$)',
        lambda match: match[1] + version + match[2],
        manifest,
    )
    lock, lock_count = re.subn(
        r'(?m)(^\[\[package\]\]\nname = "changenote"\nversion = ")[^"]+("$)',
        lambda match: match[1] + version + match[2],
        lock,
    )
    if manifest_count != 1 or lock_count != 1:
        raise ValueError("Cannot locate the Cargo package version fields")
    manifest_path.write_text(manifest)
    lock_path.write_text(lock)
    return version


if __name__ == "__main__":
    print(prepare(
        Path(sys.argv[1]), os.environ["EXPECTED_RELEASE_COMMIT"],
        json.loads(os.environ["CHANGENOTE_RELEASES"]),
    ))
