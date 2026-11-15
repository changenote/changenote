"""Check that a Python source archive contains its declared license files."""

import argparse
import email.parser
from pathlib import Path
import tarfile


def check_sdist(path: Path) -> None:
    root = path.name.removesuffix(".tar.gz")
    with tarfile.open(path, "r:gz") as archive:
        with archive.extractfile(f"{root}/PKG-INFO") as package_info:
            metadata = email.parser.BytesParser().parse(package_info)
        licenses = metadata.get_all("License-File", [])
        if not licenses:
            raise ValueError(f"{path}: PKG-INFO must declare a License-File")
        files = {
            member.name for member in archive.getmembers() if member.isfile()
        }
        for license_file in licenses:
            if f"{root}/{license_file}" not in files:
                raise ValueError(f"{path}: missing license file {license_file}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archives", type=Path, nargs="+")
    for archive_path in parser.parse_args().archives:
        check_sdist(archive_path)
