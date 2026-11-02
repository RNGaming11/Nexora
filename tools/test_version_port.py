import json
import os
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


class JavaRuntimeSelectionTests(unittest.TestCase):
    def test_matching_installed_java_is_kept(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "jdk-21"
            executable = home / "bin" / ("java.exe" if os.name == "nt" else "java")
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"")
            with patch.dict(os.environ, {"JAVA_HOME": str(home)}, clear=True), \
                 patch.object(version_port.shutil, "which", return_value=None), \
                 patch.object(version_port, "java_major", return_value=21), \
                 patch.object(version_port, "install_mojang_java") as install:
                self.assertEqual(version_port.java_command(21), str(executable))
                install.assert_not_called()

    def test_newer_java_is_not_used_for_1_21_1(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "jdk-25"
            executable = home / "bin" / ("java.exe" if os.name == "nt" else "java")
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"")
            mojang_java = Path(directory) / "mojang-java-21"
            with patch.dict(os.environ, {"JAVA_HOME": str(home)}, clear=True), \
                 patch.object(version_port.shutil, "which", return_value=None), \
                 patch.object(version_port, "java_major", return_value=25), \
                 patch.object(version_port, "install_mojang_java", return_value=mojang_java) as install:
                self.assertEqual(version_port.java_command(21), str(mojang_java))
                install.assert_called_once_with(21)

    def test_cached_mojang_java_is_reused(self):
        with tempfile.TemporaryDirectory() as directory:
            component = version_port.JAVA_RUNTIME_COMPONENTS[21]
            home = Path(directory) / "runtimes" / component / "windows-x64" / component
            executable = home / "bin" / ("java.exe" if os.name == "nt" else "java")
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"")
            with patch.object(version_port, "STORE", Path(directory)), \
                 patch.object(version_port, "mojang_runtime_platform", return_value="windows-x64"), \
                 patch.dict(os.environ, {}, clear=True), \
                 patch.object(version_port.shutil, "which", return_value=None), \
                 patch.object(version_port, "java_major", return_value=21), \
                 patch.object(version_port, "install_mojang_java") as install:
                self.assertEqual(version_port.java_command(21), str(executable))
                install.assert_not_called()

    def test_26_3_can_still_select_java_25(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "jdk-25"
            executable = home / "bin" / ("java.exe" if os.name == "nt" else "java")
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"")
            with patch.dict(os.environ, {"JAVA_HOME": str(home)}, clear=True), \
                 patch.object(version_port.shutil, "which", return_value=None), \
                 patch.object(version_port, "java_major", return_value=25):
                self.assertEqual(version_port.java_command(25), str(executable))


if __name__ == "__main__":
    unittest.main()
