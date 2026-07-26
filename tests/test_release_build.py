from __future__ import annotations

import tempfile
import unittest
import zipfile
from pathlib import Path

from repopact import release_build


class ReleaseBuildTests(unittest.TestCase):
    def _wheel(self, root: Path, *, flat_module: bool = False) -> Path:
        path = root / "repopact-3.0.0-py3-none-any.whl"
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr("repopact/__init__.py", "")
            for index in range(release_build.EXPECTED_SCHEMAS):
                archive.writestr(f"repopact/schemas/{index}.json", "{}")
            for index in range(release_build.EXPECTED_TEMPLATES):
                archive.writestr(f"repopact/templates/{index}.txt", "")
            archive.writestr("repopact-3.0.0.dist-info/top_level.txt", "repopact\n")
            archive.writestr("repopact-3.0.0.dist-info/METADATA", "Version: 3.0.0\n")
            if flat_module:
                archive.writestr("frontmatter.py", "")
        return path

    def test_clean_package_wheel_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            report = release_build.inspect_wheel(
                self._wheel(Path(temporary)),
                "3.0.0",
            )
        self.assertEqual(["repopact"], report["import_roots"])
        self.assertEqual(0, report["data_files"])

    def test_stale_flat_module_is_rejected_even_when_top_level_txt_is_clean(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = self._wheel(Path(temporary), flat_module=True)
            with self.assertRaisesRegex(
                release_build.ReleaseBuildError,
                "import roots.*frontmatter.py",
            ):
                release_build.inspect_wheel(path, "3.0.0")


if __name__ == "__main__":
    unittest.main()
