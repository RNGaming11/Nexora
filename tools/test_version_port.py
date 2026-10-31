import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import version_port


class PreparedPathTests(unittest.TestCase):
    def test_paths_accepts_legacy_top_level_report_path(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            reports = root / "datagen" / "reports"
            record = {
                "paths": {"client": str(root / "client.jar")},
                "reports": str(reports),
            }
            (root / "prepared.json").write_text(json.dumps(record), encoding="utf-8")

            with patch.object(version_port, "version_dir", return_value=root):
                prepared = version_port.paths("1.21.1")

            self.assertEqual(prepared["client"], root / "client.jar")
            self.assertEqual(prepared["reports"], reports)

    def test_paths_prefers_canonical_report_path(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            canonical = root / "datagen" / "reports"
            legacy = root / "old-reports"
            record = {
                "paths": {"reports": str(canonical)},
                "reports": str(legacy),
            }
            (root / "prepared.json").write_text(json.dumps(record), encoding="utf-8")

            with patch.object(version_port, "version_dir", return_value=root):
                prepared = version_port.paths("1.21.1")

            self.assertEqual(prepared["reports"], canonical)


if __name__ == "__main__":
    unittest.main()
