#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Check stdin bytes and cleanup retries in the CLI verification helper."""

import importlib.util
import json
import pathlib
import shlex
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock


CONTROL = pathlib.Path(__file__).resolve().with_name("control.py")
REPO = CONTROL.parents[4]
SPEC = importlib.util.spec_from_file_location("control", CONTROL)
control = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(control)


def run_control(*args, data=b"", deadline=None):
    """Bound each stdin drive so a broken terminal path cannot hang the tests."""
    command = ["uv", "run", "--script", str(CONTROL), *map(str, args)]
    if deadline is not None:
        command = ["timeout", "--kill-after=2s", f"{deadline}s", *command]
    return subprocess.run(
        command, cwd=REPO, input=data, capture_output=True, check=False,
        timeout=300,
    )


class StdinTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not shutil.which("timeout"):
            raise RuntimeError("stdin regression tests require GNU timeout")
        launched = run_control("launch")
        if launched.returncode:
            raise RuntimeError(launched.stderr.decode(errors="replace"))
        cls.evidence = pathlib.Path(launched.stdout.decode().strip())
        cls.addClassCleanup(cls.cleanup)
        metadata = json.loads((cls.evidence / "run.json").read_text())
        cls.scratch = pathlib.Path(metadata["scratch"])
        cls.fixture = cls.scratch / "project"
        (cls.evidence / "proof.md").write_text(
            "# Stdin regression proof\n\n"
            "The tests use uv run --script, the real CLI, and util-linux script.\n"
            "GNU timeout bounds each input drive to 10 seconds.\n"
        )
        print(f"Evidence: {cls.evidence}", flush=True)

    @classmethod
    def cleanup(cls):
        result = run_control("cleanup", cls.evidence)
        if result.returncode:
            raise RuntimeError(result.stderr.decode(errors="replace"))
        metadata = json.loads((cls.evidence / "run.json").read_text())
        if pathlib.Path(metadata["scratch"]).exists():
            raise AssertionError("cleanup left scratch state behind")
        if not (cls.evidence / "cleanup.json").exists():
            raise AssertionError("cleanup did not retain its result")
        if not (cls.evidence / "proof.md").exists():
            raise AssertionError("cleanup removed the proof")

    def check_message(self, name, data):
        result = run_control(
            "drive", self.evidence, name, "add", name, "--patch", "verify-pkg",
            "--message-file", "-", data=data, deadline=10,
        )
        self.assertEqual(
            result.returncode, 0,
            (result.stdout + result.stderr).decode(errors="replace"),
        )
        step = next(self.evidence.glob(f"[0-9][0-9][0-9]-{name}"))
        self.assertEqual((step / "stdin.txt").read_bytes(), data)
        record = json.loads((step / "command.json").read_text())
        self.assertEqual(record["exit_code"], 0)
        self.assertEqual(record["stdin_file"], str(step / "stdin.txt"))
        self.assertEqual(
            record["shell_command"], shlex.join(record["command"])
            + " < " + shlex.quote(record["stdin_file"]),
        )
        self.assertIn(name, (step / "terminal.txt").read_text())
        self.assertTrue((step / "after/sha256.json").is_file())
        expected = (
            b"---\nverify-pkg: patch\n---\n\n" + data.strip() + b"\n"
        )
        path = pathlib.Path(".changenote") / f"{name}.md"
        self.assertEqual((self.fixture / path).read_bytes(), expected)
        self.assertEqual((step / "after/files" / path).read_bytes(), expected)

        label = f"{name}-status"
        status = run_control("drive", self.evidence, label, "status")
        self.assertEqual(status.returncode, 0, status.stderr.decode())
        status_step = next(self.evidence.glob(f"[0-9][0-9][0-9]-{label}"))
        plan = json.loads((status_step / "output.txt").read_text())
        self.assertEqual(len(plan), 1)
        self.assertIn(
            data.decode().replace("\r\n", "\n").strip(), plan[0]["notes"]
        )
        self.assertEqual(
            (status_step / "before/sha256.json").read_bytes(),
            (status_step / "after/sha256.json").read_bytes(),
        )
        self.assertEqual((self.fixture / "VERSION").read_bytes(), b"1.2.3\n")
        self.assertFalse((self.fixture / "CHANGELOG.md").exists())
        self.assertFalse((self.fixture / ".changenote/releases.json").exists())
        with (self.evidence / "proof.md").open("a") as proof:
            proof.write(
                f"\nPASS: {name}; {len(data)} exact input bytes; note bytes, "
                "terminal evidence, and read-only status checked.\n"
            )

    def test_message_without_final_newline(self):
        self.check_message("stdin-no-newline", b"Fix behavior.")

    def test_line_larger_than_terminal_limit(self):
        self.check_message("stdin-long-line", b"x" * 5000 + b"\n")

    def test_unicode_crlf_and_terminal_control_bytes(self):
        self.check_message(
            "stdin-control-bytes",
            "Keep café 漢字.\r\n\r\nKeep \t tabs, \x03, \x04, and \x7f bytes.\n".encode(),
        )


class CleanupTests(unittest.TestCase):
    def setUp(self):
        control.ARTIFACTS.mkdir(exist_ok=True)
        self.evidence = pathlib.Path(tempfile.mkdtemp(
            prefix="cli-changenote-", dir=control.ARTIFACTS,
        ))
        self.scratch = pathlib.Path(tempfile.mkdtemp(
            prefix="changenote-cli-",
        )).resolve()
        self.owner = self.scratch / "owner.txt"
        self.owner.write_text(str(self.evidence))
        self.project = self.scratch / "project"
        self.project.mkdir()
        (self.project / "VERSION").write_bytes(b"1.2.3\n")
        control.save_json(self.evidence / "run.json", {
            "repo": str(REPO), "scratch": str(self.scratch),
        })
        (self.evidence / "proof.md").write_text(
            f"# Cleanup regression case\n\n{self.id()}\n"
        )
        self.addCleanup(self.finish)

    def finish(self):
        # Restore only the marker removed by this test's fault setup.
        if self.scratch.exists() and not self.owner.exists():
            self.owner.write_text(str(self.evidence))
        control.cleanup(self.evidence)
        self.assertFalse(self.scratch.exists())
        self.assertTrue((self.evidence / "proof.md").is_file())
        self.assertTrue((self.evidence / "cleanup.json").is_file())
        self.assertEqual(
            (self.evidence / "final/files/VERSION").read_bytes(), b"1.2.3\n"
        )

    def test_completed_snapshot_is_reused(self):
        control.snapshot(self.project, self.evidence / "final")
        inventory = (self.evidence / "final/sha256.json").read_bytes()
        with mock.patch.object(control, "snapshot", side_effect=AssertionError):
            control.cleanup(self.evidence)
        self.assertEqual(
            (self.evidence / "final/sha256.json").read_bytes(), inventory
        )

    def test_incomplete_snapshot_is_retained(self):
        partial = self.evidence / "final/files"
        partial.mkdir(parents=True)
        (partial / "partial.txt").write_bytes(b"old proof")
        (self.evidence / "final/sha256.json").write_bytes(b"{incomplete")
        control.cleanup(self.evidence)
        saved = list(self.evidence.glob(
            ".final-incomplete-*/snapshot/files/partial.txt"
        ))
        self.assertEqual(len(saved), 1)
        self.assertEqual(saved[0].read_bytes(), b"old proof")

    def test_failed_snapshot_can_retry(self):
        def fail_snapshot(fixture, destination):
            (destination / "files").mkdir()
            (destination / "files/partial.txt").write_bytes(b"partial proof")
            raise OSError("test snapshot failure")

        with mock.patch.object(control, "snapshot", side_effect=fail_snapshot):
            with self.assertRaisesRegex(OSError, "test snapshot failure"):
                control.cleanup(self.evidence)
        self.assertFalse((self.evidence / "final").exists())
        self.assertEqual(self.owner.read_text(), str(self.evidence))
        control.cleanup(self.evidence)
        saved = list(self.evidence.glob(".final-*/files/partial.txt"))
        self.assertEqual(len(saved), 1)
        self.assertEqual(saved[0].read_bytes(), b"partial proof")

    def test_partial_project_removal_can_retry(self):
        extra = self.project / "extra.txt"
        extra.write_bytes(b"saved before removal")
        remove = shutil.rmtree

        def fail_removal(path, *args, **kwargs):
            if pathlib.Path(path) == self.project:
                extra.unlink()
                raise OSError("test project removal failure")
            return remove(path, *args, **kwargs)

        with mock.patch.object(control.shutil, "rmtree", side_effect=fail_removal):
            with self.assertRaisesRegex(OSError, "test project removal failure"):
                control.cleanup(self.evidence)
        self.assertEqual(self.owner.read_text(), str(self.evidence))
        inventory = (self.evidence / "final/sha256.json").read_bytes()
        control.cleanup(self.evidence)
        self.assertEqual(
            (self.evidence / "final/sha256.json").read_bytes(), inventory
        )
        self.assertEqual(
            (self.evidence / "final/files/extra.txt").read_bytes(),
            b"saved before removal",
        )

    def test_root_removal_failure_restores_owner(self):
        with mock.patch.object(pathlib.Path, "rmdir", side_effect=OSError(
            "test root removal failure"
        )):
            with self.assertRaisesRegex(OSError, "test root removal failure"):
                control.cleanup(self.evidence)
        self.assertEqual(self.owner.read_text(), str(self.evidence))
        self.assertFalse(self.project.exists())
        control.cleanup(self.evidence)

    def set_finalizing(self):
        control.save_json(self.evidence / "cleanup.json", {
            "scratch": str(self.scratch), "removed": False, "finalizing": True,
        })
        self.owner.unlink()

    def test_interrupted_marker_removal_can_retry(self):
        control.final_snapshot(self.project, self.evidence)
        shutil.rmtree(self.project)
        self.set_finalizing()
        with self.assertRaises(FileNotFoundError):
            control.load_run(self.evidence)
        control.cleanup(self.evidence)
        self.assertFalse(self.scratch.exists())

    def test_missing_owner_cannot_authorize_nonempty_state(self):
        self.set_finalizing()
        with self.assertRaisesRegex(ValueError, "missing scratch ownership"):
            control.cleanup(self.evidence)
        self.assertEqual((self.project / "VERSION").read_bytes(), b"1.2.3\n")


if __name__ == "__main__":
    unittest.main(verbosity=2)
