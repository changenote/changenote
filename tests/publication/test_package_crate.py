"""Check standalone crate assembly without registry access or Cargo uploads."""

import importlib.util
import os
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("package_crate", ROOT / "scripts/package_crate.py")
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)


class PackageCrate(unittest.TestCase):
    def test_bundle_contains_unchanged_implementation_sources_and_resources(self):
        manifest = ROOT / "crates/changenote/Cargo.toml"
        lockfile = ROOT / "Cargo.lock"
        original_manifest, original_lockfile = manifest.read_bytes(), lockfile.read_bytes()
        with packaging.prepared_crate(ROOT) as crate:
            self.assertFalse(crate.is_relative_to(ROOT))
            for source in (ROOT / "crates/changenote-core/src").iterdir():
                self.assertEqual(source.read_bytes(), (crate / "src" / source.name).read_bytes())
            self.assertEqual(
                (ROOT / "crates/changenote-cli/src/lib.rs").read_bytes(),
                (crate / "src/cli.rs").read_bytes(),
            )
            self.assertEqual(
                (ROOT / "crates/changenote-skill/src/lib.rs").read_bytes(),
                (crate / "src/skill.rs").read_bytes(),
            )
            self.assertEqual(
                (ROOT / "crates/changenote-skill/SKILL.md").read_bytes(),
                (crate / "SKILL.md").read_bytes(),
            )
            for source in (ROOT / "crates/changenote-core/tests").glob("*.rs"):
                self.assertEqual(
                    source.read_text().replace("use changenote_core as changenote;\n", ""),
                    (crate / "tests" / f"core_{source.name}").read_text(),
                )
            self.assertIn("extern crate changenote as changenote_core;", (crate / "src/main.rs").read_text())
            self.assertTrue((crate / "src/main.rs").read_bytes().endswith(
                (ROOT / "crates/changenote/src/main.rs").read_bytes()
            ))
            for path in (ROOT / "crates/changenote").rglob("*"):
                if path.is_file() and path.parent.name != "src" and path.name != "Cargo.toml":
                    relative = path.relative_to(ROOT / "crates/changenote")
                    if relative == Path("tests/cli.rs"):
                        expected = (path.read_text()
                                    .replace("changenote_skill::SKILL", 'include_str!("../SKILL.md")')
                                    .replace("../../changenote-cli/tests/e2e/mod.rs", "cli/mod.rs")
                                    .replace("../changenote-cli/tests/e2e/fixtures/planning",
                                             "tests/cli/fixtures/planning"))
                        self.assertEqual(expected, (crate / relative).read_text())
                    else:
                        self.assertEqual(path.read_bytes(), (crate / relative).read_bytes())
            suite = ROOT / "crates/changenote-cli/tests/e2e"
            for source in suite.rglob("*"):
                if source.is_file():
                    self.assertEqual(
                        source.read_bytes(),
                        (crate / "tests/cli" / source.relative_to(suite)).read_bytes(),
                    )

            bundled = tomllib.loads((crate / "Cargo.toml").read_text())
            public = tomllib.loads(original_manifest.decode())
            self.assertEqual(bundled["package"], public["package"])
            for section in ("dependencies", "dev-dependencies"):
                for value in bundled[section].values():
                    if isinstance(value, dict):
                        self.assertNotIn("path", value)
                        self.assertNotIn("workspace", value)
                self.assertTrue(set(packaging.PRIVATE_CRATES).isdisjoint(bundled[section]))
            for name in packaging.PRIVATE_CRATES:
                private = tomllib.loads((ROOT / "crates" / name / "Cargo.toml").read_text())
                for section in ("dependencies", "dev-dependencies"):
                    for dependency, value in private.get(section, {}).items():
                        if dependency not in packaging.PRIVATE_CRATES:
                            self.assertEqual(bundled[section][dependency], value)

            before = tomllib.loads(original_lockfile.decode())["package"]
            after = tomllib.loads((crate / "Cargo.lock").read_text())["package"]
            self.assertEqual(
                [package for package in before if package["name"] not in {"changenote", *packaging.PRIVATE_CRATES}],
                [package for package in after if package["name"] != "changenote"],
            )
            self.assertFalse(any(package["name"] in packaging.PRIVATE_CRATES for package in after))
            locked = next(package for package in after if package["name"] == "changenote")
            expected = {
                value.get("package", name) if isinstance(value, dict) else name
                for section in ("dependencies", "dev-dependencies")
                for name, value in bundled[section].items()
            }
            self.assertEqual(set(locked["dependencies"]), expected)
        self.assertFalse(crate.exists())
        self.assertEqual(manifest.read_bytes(), original_manifest)
        self.assertEqual(lockfile.read_bytes(), original_lockfile)

    def test_legacy_release_layouts_are_not_rewritten(self):
        for member in (False, True):
            with self.subTest(member=member), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                manifest = root / ("crates/changenote/Cargo.toml" if member else "Cargo.toml")
                manifest.parent.mkdir(parents=True, exist_ok=True)
                text = '[package]\nname = "changenote"\nversion = "1.2.3"\n'
                manifest.write_text(text)
                with packaging.prepared_crate(root) as prepared:
                    self.assertEqual(prepared, root.resolve())
                self.assertEqual(manifest.read_text(), text)

    def test_conflicting_and_unresolved_dependencies_fail(self):
        public = '[dependencies]\nanyhow = "1"\n\n[dev-dependencies]\n'
        for value in ("2", {"path": "../other"}, {"workspace": True}):
            with self.subTest(value=value), self.assertRaises(ValueError):
                packaging.bundled_manifest(public, [{"dependencies": {"anyhow": value}}])

    def test_invalid_local_lockfile_packages_fail(self):
        original = (ROOT / "Cargo.lock").read_text()
        for lock in (
            original.replace('name = "changenote-core"', 'name = "other"'),
            original + '\n[[package]]\nname = "changenote-core"\nversion = "0.0.0"\n',
            original.replace('name = "changenote-core"', 'source = "registry+example"\nname = "changenote-core"'),
        ):
            with self.subTest(lock=lock[-120:]), self.assertRaises(ValueError):
                packaging.bundled_lockfile(lock)

    def test_cargo_runs_in_temporary_crate_and_preserves_explicit_target(self):
        for target, expected in (
            ("/tmp/changenote-test-target", "/tmp/changenote-test-target"),
            ("custom-target", str(ROOT / "custom-target")),
        ):
            with self.subTest(target=target), patch.object(packaging.subprocess, "run") as cargo, patch.dict(
                os.environ, {"CARGO_TARGET_DIR": target}
            ):
                packaging.run_cargo(ROOT, ["package", "--locked", "-p", "changenote"])
            args, kwargs = cargo.call_args
            self.assertEqual(args, (["cargo", "package", "--locked", "-p", "changenote"],))
            self.assertFalse(kwargs["cwd"].exists())
            self.assertEqual(kwargs["env"]["CARGO_TARGET_DIR"], expected)
            self.assertIs(kwargs["check"], True)

    def test_cargo_failure_cleans_up_and_does_not_change_sources(self):
        original = (ROOT / "Cargo.lock").read_bytes()
        with patch.object(packaging.subprocess, "run") as cargo, patch.dict(os.environ):
            os.environ.pop("CARGO_TARGET_DIR", None)
            cargo.side_effect = subprocess.CalledProcessError(1, "cargo package")
            with self.assertRaises(subprocess.CalledProcessError):
                packaging.run_cargo(ROOT, ["package", "--locked"])
        self.assertFalse(cargo.call_args.kwargs["cwd"].exists())
        self.assertEqual(cargo.call_args.kwargs["env"]["CARGO_TARGET_DIR"], str(ROOT / "target"))
        self.assertEqual((ROOT / "Cargo.lock").read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
