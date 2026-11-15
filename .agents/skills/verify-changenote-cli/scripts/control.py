#!/usr/bin/env -S uv run --no-project python
"""Drive the source CLI in disposable projects and retain terminal evidence."""

import argparse
import hashlib
import json
import pathlib
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import tomllib


REPO = pathlib.Path(__file__).resolve().parents[4]
ARTIFACTS = REPO / ".verification"
ALLOWED = {
    "init", "add", "status", "check", "doctor", "release-notes",
    "skill", "completion", "--help", "--version",
}
READ_ONLY = ALLOWED - {"init", "add"}


def save_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def load_run(path):
    """Accept only a run owned by this checkout and its marked scratch root."""
    run = pathlib.Path(path).resolve()
    if run.parent != ARTIFACTS or not run.name.startswith("cli-changenote-"):
        raise ValueError("not a verification run in this checkout")
    metadata = json.loads((run / "run.json").read_text(encoding="utf-8"))
    scratch = pathlib.Path(metadata["scratch"])
    if (
        metadata["repo"] != str(REPO)
        or scratch != scratch.resolve()
        or scratch.parent != pathlib.Path(tempfile.gettempdir()).resolve()
        or not scratch.name.startswith("changenote-cli-")
    ):
        raise ValueError("run ownership mismatch")
    if scratch.exists():
        owner = (scratch / "owner.txt").read_text(encoding="utf-8")
        if owner != str(run):
            raise ValueError("scratch ownership mismatch; do not remove it")
    elif not (run / "cleanup.json").exists():
        raise ValueError("scratch root is missing without a cleanup record")
    return run, scratch


def snapshot(fixture, destination):
    """Preserve file bytes and hashes, including local Git refs when present."""
    hashes = {}
    for path in sorted(fixture.rglob("*")):
        if path.is_symlink():
            raise ValueError("fixture symlinks are not supported")
        if path.is_file():
            hashes[path.relative_to(fixture).as_posix()] = hashlib.sha256(
                path.read_bytes()
            ).hexdigest()
    shutil.copytree(fixture, destination / "files")
    save_json(destination / "sha256.json", hashes)
    return hashes


def drive(run_path, label, args, emit=True):
    """Run one allowlisted user command in a fresh util-linux script PTY."""
    run, scratch = load_run(run_path)
    if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", label):
        raise ValueError("use a lowercase kebab-case evidence label")
    if not args or args[0] not in ALLOWED:
        raise ValueError(
            "command not allowed; version, tag, and help dispatch are excluded"
        )
    if any(arg == "--root" or arg.startswith("--root=") for arg in args):
        raise ValueError("the helper owns --root; do not override it")
    fixture = scratch / "project"
    if not fixture.is_dir() or fixture.is_symlink():
        raise ValueError("fixture is missing or unsafe; launch a new run")
    number = len(list(run.glob("[0-9][0-9][0-9]-*")))
    step = run / f"{number:03d}-{label}"
    step.mkdir()
    command = [
        "cargo", "run", "--locked", "--quiet", "-p", "changenote", "--",
        "--root", str(fixture), *args,
    ]
    record = {"command": command, "cwd": str(REPO)}
    save_json(step / "command.json", record)
    before = snapshot(fixture, step / "before")
    stdin = b""
    if (
        "--message-file" in args
        and args[args.index("--message-file") + 1:] == ["-"]
    ):
        if sys.stdin.isatty():
            raise ValueError("pipe the Markdown message; do not open a prompt")
        stdin = sys.stdin.buffer.read()
        (step / "stdin.txt").write_bytes(stdin)
    result = subprocess.run(
        ["script", "--quiet", "--return", "--echo", "never", "--command",
         shlex.join(command), str(step / "terminal.txt")],
        cwd=REPO, input=stdin, capture_output=True, check=False,
    )
    output = result.stdout.replace(b"\r\n", b"\n")
    (step / "output.txt").write_bytes(output)
    (step / "script-stderr.txt").write_bytes(result.stderr)
    record["exit_code"] = result.returncode
    save_json(step / "command.json", record)
    after = snapshot(fixture, step / "after")
    if emit:
        sys.stdout.buffer.write(output)
        sys.stderr.buffer.write(result.stderr)
        print(f"Evidence: {step}", file=sys.stderr)
    if args[0] in READ_ONLY and before != after:
        raise ValueError(f"read-only command changed fixture files: {step}")
    return result.returncode, output


def doctor(run_path, emit=True):
    """Check the checkout's CLI version and the fixture's local readiness."""
    run, _ = load_run(run_path)
    manifest = tomllib.loads(
        (REPO / "crates/changenote/Cargo.toml").read_text(encoding="utf-8")
    )
    code, output = drive(run, "doctor-build", ["--version"], emit)
    expected = f"changenote {manifest['package']['version']}\n".encode()
    if code or output != expected:
        raise ValueError("CLI build version does not match the public package")
    code, output = drive(run, "doctor", ["doctor"], emit)
    if code or not json.loads(output)["passed"]:
        raise ValueError("fixture doctor failed; inspect its evidence")


def cleanup(run_path):
    """Remove only owned scratch state; leave the evidence in place."""
    run, scratch = load_run(run_path)
    if scratch.exists():
        if (scratch / "project").is_dir():
            snapshot(scratch / "project", run / "final")
        shutil.rmtree(scratch)
    save_json(run / "cleanup.json", {"scratch": str(scratch), "removed": True})
    print(f"Evidence retained: {run}", file=sys.stderr)


def launch():
    """Build once and initialize a usable project from an empty directory."""
    for tool in ["cargo", "git", "script"]:
        if not shutil.which(tool):
            raise ValueError(f"missing prerequisite: {tool}")
    check = subprocess.run(
        ["script", "--version"], capture_output=True, check=True
    )
    if b"util-linux" not in check.stdout:
        raise ValueError("this helper requires the util-linux script command")
    ARTIFACTS.mkdir(exist_ok=True)
    run = pathlib.Path(tempfile.mkdtemp(prefix="cli-changenote-", dir=ARTIFACTS))
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="changenote-cli-")).resolve()
    (scratch / "owner.txt").write_text(str(run), encoding="utf-8")
    (scratch / "project").mkdir()
    save_json(run / "run.json", {"repo": str(REPO), "scratch": str(scratch)})
    print(f"Run: {run}", file=sys.stderr)
    try:
        with (run / "source.txt").open("wb") as log:
            for args in [
                ["rev-parse", "HEAD"], ["status", "--short"], ["diff", "HEAD"]
            ]:
                subprocess.run(["git", *args], cwd=REPO, stdout=log, check=True)
        with (run / "build.txt").open("wb") as log:
            subprocess.run(
                ["cargo", "build", "--locked", "-p", "changenote"],
                cwd=REPO, stdout=log, stderr=subprocess.STDOUT, check=True,
            )
        code, _ = drive(
            run, "init", ["init", "--name", "verify-pkg", "--version", "1.2.3"],
            emit=False,
        )
        if code:
            raise ValueError("initialization failed")
        doctor(run, emit=False)
    except BaseException:
        cleanup(run)
        raise
    print(run)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_subparsers(dest="action", required=True)
    actions.add_parser("launch")
    for action in ["doctor", "cleanup", "drive"]:
        command = actions.add_parser(action)
        command.add_argument("run", type=pathlib.Path)
        if action == "drive":
            command.add_argument("label")
            command.add_argument("args", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.action == "launch":
        launch()
    elif args.action == "doctor":
        doctor(args.run)
    elif args.action == "cleanup":
        cleanup(args.run)
    else:
        return drive(args.run, args.label, args.args)[0]
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"Verification failed: {error}", file=sys.stderr)
        sys.exit(1)
