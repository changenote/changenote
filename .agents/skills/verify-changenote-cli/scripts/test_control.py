#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Check piped Markdown with the real CLI and util-linux terminal harness."""

import json
import pathlib
import shlex
import shutil
import subprocess
import unittest


CONTROL = pathlib.Path(__file__).resolve().with_name("control.py")
REPO = CONTROL.parents[4]


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


if __name__ == "__main__":
    unittest.main(verbosity=2)
