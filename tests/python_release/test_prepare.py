"""Check all release layouts without depending on the live Cargo manifest."""

import json
import os
import runpy
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
prepare = runpy.run_path(str(ROOT / "scripts/prepare-python-release.py"))["prepare"]
SOURCES = ("VERSION", "Cargo.toml", "crates/changenote/Cargo.toml")


class Prepare(unittest.TestCase):
    def fixture(self, source):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.manifest = self.root / (
            "Cargo.toml" if source == "VERSION" else source
        )
        self.manifest.parent.mkdir(parents=True, exist_ok=True)
        version = "0.9.0" if source == "VERSION" else "1.2.3"
        self.manifest.write_text(
            '[package]\nname = "changenote"\n'
            f'version = "{version}"\nedition = "2024"\n'
            '[dependencies]\nexample = "0.9.0"\n'
        )
        if source == "crates/changenote/Cargo.toml":
            (self.root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["crates/changenote"]\nresolver = "3"\n'
            )
        (self.root / "Cargo.lock").write_text(
            'version = 4\n\n[[package]]\nname = "changenote"\n'
            f'version = "{version}"\n\n'
            '[[package]]\nname = "example"\nversion = "0.9.0"\n'
            'source = "registry+https://example.invalid/index"\n'
        )
        (self.root / ".changenote").mkdir()
        self.config({"path": ".", "version_file": source})
        self.batch([{"package": "changenote", "version": "1.2.3", "tag": "v1.2.3"}])
        if source == "VERSION":
            (self.root / "VERSION").write_text("1.2.3\n")
        self.git("init", "-q")
        self.git("add", "--", ".")
        self.git("commit", "-qm", "fixture")
        self.release_commit = self.git("rev-parse", "HEAD")

    def config(self, source):
        (self.root / ".changenote/config.json").write_text(
            json.dumps({"packages": {"changenote": source}})
        )

    def git(self, *args):
        env = dict(os.environ, GIT_AUTHOR_NAME="Fixture", GIT_COMMITTER_NAME="Fixture",
                   GIT_AUTHOR_EMAIL="fixture@example.invalid", GIT_COMMITTER_EMAIL="fixture@example.invalid")
        return subprocess.check_output(["git", "-C", str(self.root), *args], env=env, text=True).strip()

    def batch(self, batch):
        self.batch_data = batch

    def prepare(self):
        return prepare(self.root, self.release_commit, self.batch_data)

    def snapshot(self):
        return {
            str(path.relative_to(self.root)): path.read_bytes()
            for path in self.root.rglob("*") if path.is_file()
        }

    def assert_rejected(self):
        before = self.snapshot()
        with self.assertRaises(ValueError):
            self.prepare()
        self.assertEqual(self.snapshot(), before)

    def test_commit_and_metadata_mismatches_do_not_write(self):
        self.fixture("crates/changenote/Cargo.toml")
        for commit in ("not-a-sha", "0" * 40):
            with self.subTest(commit=commit):
                self.release_commit = commit
                self.assert_rejected()

    def test_shallow_file_free_source_needs_no_parent_history(self):
        self.fixture("crates/changenote/Cargo.toml")
        original = self.root
        target = Path(self.enterContext(tempfile.TemporaryDirectory())) / "source"
        subprocess.run(["git", "clone", "--quiet", "--depth=1", original.as_uri(), str(target)], check=True)
        self.root = target
        self.assertEqual(self.git("rev-parse", "--is-shallow-repository"), "true")
        before = self.snapshot()
        self.assertEqual(self.prepare(), "1.2.3")
        self.assertFalse((self.root / ".changenote/releases.json").exists())
        self.assertEqual(self.snapshot(), before)

    def test_malformed_verified_metadata_does_not_write(self):
        self.fixture("Cargo.toml")
        for value in (None, {}, [None], [{}], [{"package": "changenote", "version": "1.2.3", "tag": ""}]):
            with self.subTest(value=value):
                self.batch(value)
                self.assert_rejected()

    def test_legacy_only_package_versions_change_and_repeat_is_safe(self):
        self.fixture("VERSION")
        before = self.snapshot()
        original_manifest = tomllib.loads(self.manifest.read_text())
        original_lock = tomllib.loads((self.root / "Cargo.lock").read_text())
        self.assertEqual(self.prepare(), "1.2.3")
        original_manifest["package"]["version"] = "1.2.3"
        original_lock["package"][0]["version"] = "1.2.3"
        self.assertEqual(tomllib.loads(self.manifest.read_text()), original_manifest)
        self.assertEqual(
            tomllib.loads((self.root / "Cargo.lock").read_text()), original_lock
        )
        for name, content in before.items():
            if name not in ("Cargo.toml", "Cargo.lock"):
                self.assertEqual((self.root / name).read_bytes(), content)
        first = self.snapshot()
        self.assertEqual(self.prepare(), "1.2.3")
        self.assertEqual(self.snapshot(), first)

    def test_cargo_sources_without_version_file_are_read_only(self):
        for source in SOURCES[1:]:
            with self.subTest(source=source):
                self.fixture(source)
                before = self.snapshot()
                self.assertEqual(self.prepare(), "1.2.3")
                self.assertEqual(self.prepare(), "1.2.3")
                self.assertEqual(self.snapshot(), before)
                self.assertFalse((self.root / "VERSION").exists())

    def test_invalid_versions_do_not_write(self):
        for source in SOURCES:
            for version in ("01.2.3", "1.2", "1.2.3-rc.1", "1.2.3+build"):
                with self.subTest(source=source, version=version):
                    self.fixture(source)
                    if source == "VERSION":
                        (self.root / "VERSION").write_text(version)
                    else:
                        self.manifest.write_text(self.manifest.read_text().replace(
                            'version = "1.2.3"', f'version = "{version}"', 1
                        ))
                    self.assert_rejected()

    def test_multiple_legacy_version_lines_do_not_write(self):
        self.fixture("VERSION")
        (self.root / "VERSION").write_text("1.2.3\n4.5.6")
        self.assert_rejected()

    def test_nonliteral_cargo_versions_do_not_write(self):
        for source in SOURCES[1:]:
            for field in ("version = 123", "version.workspace = true"):
                with self.subTest(source=source, field=field):
                    self.fixture(source)
                    self.manifest.write_text(self.manifest.read_text().replace(
                        'version = "1.2.3"', field, 1
                    ))
                    self.assert_rejected()

    def test_invalid_batches_do_not_write(self):
        for source in SOURCES:
            for batch in (
                [], [{"package": "other", "version": "1.2.3"}],
                [{"package": "changenote", "version": "1.2.4"}],
                [{"package": "changenote", "version": "1.2.3"}] * 2,
            ):
                with self.subTest(source=source, batch=batch):
                    self.fixture(source)
                    self.batch(batch)
                    self.assert_rejected()

    def test_lock_version_mismatches_do_not_write(self):
        for source in SOURCES:
            with self.subTest(source=source):
                self.fixture(source)
                lock = self.root / "Cargo.lock"
                lock.write_text(lock.read_text().replace(
                    'name = "changenote"\nversion = "',
                    'name = "changenote"\nversion = "9', 1
                ))
                self.assert_rejected()

    def test_invalid_local_packages_do_not_write(self):
        for source in SOURCES:
            for invalid in ("wrong-name", "missing", "duplicate", "registry"):
                with self.subTest(source=source, invalid=invalid):
                    self.fixture(source)
                    lock = self.root / "Cargo.lock"
                    if invalid == "wrong-name":
                        self.manifest.write_text(self.manifest.read_text().replace(
                            'name = "changenote"', 'name = "other"', 1
                        ))
                    elif invalid == "missing":
                        lock.write_text(lock.read_text().replace(
                            'name = "changenote"', 'name = "other"', 1
                        ))
                    elif invalid == "duplicate":
                        lock.write_text(lock.read_text() +
                                        '\n[[package]]\nname = "changenote"\nversion = "1.2.3"\n')
                    else:
                        lock.write_text(lock.read_text().replace(
                            'name = "changenote"\n',
                            'name = "changenote"\nsource = "registry+https://example.invalid"\n', 1
                        ))
                    self.assert_rejected()

    def test_unsupported_sources_do_not_write(self):
        for source in (
            {"path": "crates/changenote", "version_file": "Cargo.toml"},
            {"path": ".", "version_file": "../Cargo.toml"},
            {"path": ".", "version_file": "/Cargo.toml"},
            {"path": ".", "version_file": "other/Cargo.toml"},
            {"path": ".", "version_file": "crates/changenote/VERSION"},
        ):
            with self.subTest(source=source):
                self.fixture("VERSION")
                self.config(source)
                self.assert_rejected()

    def test_legacy_unlocatable_fields_do_not_write(self):
        for name in ("Cargo.toml", "Cargo.lock"):
            with self.subTest(name=name):
                self.fixture("VERSION")
                path = self.root / name
                path.write_text(path.read_text().replace(
                    'version = "0.9.0"', "version = '0.9.0'", 1
                ))
                self.assert_rejected()


if __name__ == "__main__":
    unittest.main()
