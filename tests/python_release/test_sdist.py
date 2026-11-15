"""Check license-file validation with source archive fixtures."""

import io
from pathlib import Path
import runpy
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
check_sdist = runpy.run_path(str(ROOT / "scripts/check_python_sdist.py"))[
    "check_sdist"
]


class SourceArchive(unittest.TestCase):
    def fixture(self, licenses, files):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        path = Path(temporary.name) / "changenote-1.2.3.tar.gz"
        metadata = "Metadata-Version: 2.4\nName: changenote\nVersion: 1.2.3\n"
        metadata += "".join(f"License-File: {name}\n" for name in licenses)
        files = {"PKG-INFO": metadata.encode(), **files}
        with tarfile.open(path, "w:gz") as archive:
            for name, content in files.items():
                member = tarfile.TarInfo(f"changenote-1.2.3/{name}")
                if content is None:
                    member.type = tarfile.DIRTYPE
                    archive.addfile(member)
                else:
                    member.size = len(content)
                    archive.addfile(member, io.BytesIO(content))
        return path

    def test_declared_license_files_exist(self):
        path = self.fixture(
            ["LICENSE", "licenses/NOTICE"],
            {"LICENSE": b"MIT license", "licenses/NOTICE": b"Notice"},
        )
        check_sdist(path)

    def test_nested_crate_license_does_not_replace_root_license(self):
        path = self.fixture(
            ["LICENSE"], {"crates/changenote/LICENSE": b"MIT license"}
        )
        with self.assertRaisesRegex(ValueError, "missing license file LICENSE"):
            check_sdist(path)

    def test_every_declared_license_must_exist(self):
        path = self.fixture(
            ["LICENSE", "licenses/NOTICE"], {"LICENSE": b"MIT license"}
        )
        with self.assertRaisesRegex(
            ValueError, "missing license file licenses/NOTICE"
        ):
            check_sdist(path)

    def test_license_must_be_a_regular_file(self):
        path = self.fixture(["LICENSE"], {"LICENSE": None})
        with self.assertRaisesRegex(ValueError, "missing license file LICENSE"):
            check_sdist(path)

    def test_metadata_must_declare_a_license_file(self):
        path = self.fixture([], {"LICENSE": b"MIT license"})
        with self.assertRaisesRegex(ValueError, "must declare a License-File"):
            check_sdist(path)


if __name__ == "__main__":
    unittest.main()
