"""Assemble a standalone public crate without publishing private dependencies."""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from contextlib import contextmanager
from pathlib import Path

PRIVATE_CRATES = ("changenote-core", "changenote-cli", "changenote-skill")


def toml_value(value):
    if isinstance(value, dict):
        return "{ " + ", ".join(
            f"{json.dumps(key)} = {toml_value(item)}" for key, item in value.items()
        ) + " }"
    if isinstance(value, list):
        return "[" + ", ".join(map(toml_value, value)) + "]"
    return json.dumps(value)


def bundled_manifest(manifest, implementations):
    public = tomllib.loads(manifest)
    for section in ("dependencies", "dev-dependencies"):
        dependencies = {}
        for package in (public, *implementations):
            for name, value in package.get(section, {}).items():
                if name in PRIVATE_CRATES:
                    continue
                if isinstance(value, dict) and ("path" in value or "workspace" in value):
                    raise ValueError(f"Cannot bundle non-registry dependency {name}")
                if name in dependencies and dependencies[name] != value:
                    raise ValueError(f"Conflicting dependency definitions for {name}")
                dependencies[name] = value
        replacement = f"[{section}]\n" + "".join(
            f"{json.dumps(name)} = {toml_value(value)}\n"
            for name, value in sorted(dependencies.items())
        ) + "\n"
        manifest, count = re.subn(
            rf"(?ms)^\[{section}\]\n.*?(?=^\[|\Z)",
            lambda _: replacement,
            manifest,
        )
        if count != 1:
            raise ValueError(f"Expected one [{section}] section in the public crate")
    return manifest


def bundled_lockfile(lock):
    blocks = re.split(r"(?m)(?=^\[\[package\]\]\n)", lock)
    packages = [tomllib.loads(block)["package"][0] for block in blocks[1:]]
    local_names = {"changenote", *PRIVATE_CRATES}
    for name in local_names:
        matches = [package for package in packages if package["name"] == name]
        if len(matches) != 1 or "source" in matches[0]:
            raise ValueError(f"Expected one local {name} package in Cargo.lock")
    dependencies = sorted({
        name
        for package in packages if package["name"] in local_names
        for name in package.get("dependencies", [])
        if name not in PRIVATE_CRATES
    })
    output = [blocks[0]]
    for block, package in zip(blocks[1:], packages, strict=True):
        if package["name"] in PRIVATE_CRATES:
            continue
        if package["name"] == "changenote":
            replacement = "dependencies = [\n" + "".join(
                f" {json.dumps(name)},\n" for name in dependencies
            ) + "]"
            block, count = re.subn(
                r"(?ms)^dependencies = \[\n.*?^\]", lambda _: replacement, block
            )
            if count != 1:
                raise ValueError("Expected changenote dependencies in Cargo.lock")
        output.append(block)
    return "".join(output)


@contextmanager
def prepared_crate(root):
    root = root.resolve()
    public = root / "crates/changenote"
    manifest_path = public / "Cargo.toml"
    # Older releases have either a root package or a single workspace member.
    if not manifest_path.is_file() or not any(
        name in tomllib.loads(manifest_path.read_text()).get("dependencies", {})
        for name in PRIVATE_CRATES
    ):
        yield root
        return

    implementations = []
    for name in PRIVATE_CRATES:
        package = tomllib.loads((root / "crates" / name / "Cargo.toml").read_text())
        if package["package"]["name"] != name or package["package"].get("publish") is not False:
            raise ValueError(f"Expected private implementation crate {name}")
        implementations.append(package)

    with tempfile.TemporaryDirectory(prefix="changenote-package-") as directory:
        crate = Path(directory) / "changenote"
        shutil.copytree(public, crate)
        shutil.copytree(root / "crates/changenote-core/src", crate / "src", dirs_exist_ok=True)
        shutil.copyfile(root / "crates/changenote-cli/src/lib.rs", crate / "src/cli.rs")
        shutil.copyfile(root / "crates/changenote-skill/src/lib.rs", crate / "src/skill.rs")
        shutil.copyfile(root / "crates/changenote-skill/SKILL.md", crate / "SKILL.md")
        for source in (root / "crates/changenote-core/tests").glob("*.rs"):
            # Integration tests use the public library in the standalone archive.
            (crate / "tests" / f"core_{source.name}").write_text(
                source.read_text().replace("use changenote_core as changenote;\n", "")
            )
        shutil.copytree(root / "crates/changenote-cli/tests/e2e", crate / "tests/cli")
        cli_test = crate / "tests/cli.rs"
        cli_test.write_text(
            cli_test.read_text()
            .replace("changenote_skill::SKILL", 'include_str!("../SKILL.md")')
            .replace("../../changenote-cli/tests/e2e/mod.rs", "cli/mod.rs")
            .replace(
                "../changenote-cli/tests/e2e/fixtures/planning",
                "tests/cli/fixtures/planning",
            )
        )
        main = crate / "src/main.rs"
        main.write_text(
            'extern crate changenote as changenote_core;\n'
            '#[path = "cli.rs"]\nmod changenote_cli;\n'
            '#[path = "skill.rs"]\nmod changenote_skill;\n\n' + main.read_text()
        )
        (crate / "Cargo.toml").write_text(
            bundled_manifest(manifest_path.read_text(), implementations)
        )
        (crate / "Cargo.lock").write_text(bundled_lockfile((root / "Cargo.lock").read_text()))
        yield crate


def run_cargo(root, arguments):
    root = root.resolve()
    environment = os.environ.copy()
    target = Path(environment.get("CARGO_TARGET_DIR", root / "target"))
    environment["CARGO_TARGET_DIR"] = str(target if target.is_absolute() else root / target)
    with prepared_crate(root) as crate:
        subprocess.run(["cargo", *arguments], cwd=crate, env=environment, check=True)


if __name__ == "__main__":
    run_cargo(Path.cwd(), sys.argv[1:])
