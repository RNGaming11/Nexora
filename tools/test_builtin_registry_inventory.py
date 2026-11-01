import json
import tempfile
import unittest
from pathlib import Path

from tools.generate_builtin_registry_inventory import build


class BuiltinRegistryInventoryTests(unittest.TestCase):
    def test_inventory_is_sorted_by_protocol_id_and_rejects_gaps(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "registries.json"
            report.write_text(
                json.dumps({
                    "minecraft:item": {
                        "protocol_id": 6,
                        "entries": {
                            "minecraft:stone": {"protocol_id": 1},
                            "minecraft:air": {"protocol_id": 0},
                        },
                    }
                }),
                encoding="utf-8",
            )
            inventory = build(report, "1.21.1", 3955)
            self.assertEqual(
                inventory["registries"]["minecraft:item"],
                ["minecraft:air", "minecraft:stone"],
            )
            self.assertEqual(inventory["minecraft_version"], "1.21.1")
            self.assertEqual(inventory["data_version"], 3955)
            self.assertEqual(len(inventory["source_report_sha256"]), 64)

            report.write_text(
                json.dumps({
                    "minecraft:item": {
                        "entries": {
                            "minecraft:air": {"protocol_id": 0},
                            "minecraft:stone": {"protocol_id": 2},
                        }
                    }
                }),
                encoding="utf-8",
            )
            with self.assertRaises(SystemExit):
                build(report, "1.21.1", 3955)

    def test_inventory_rejects_duplicate_ids(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "registries.json"
            report.write_text(
                json.dumps({
                    "minecraft:item": {
                        "entries": {
                            "minecraft:air": {"protocol_id": 0},
                            "minecraft:stone": {"protocol_id": 0},
                        }
                    }
                }),
                encoding="utf-8",
            )
            with self.assertRaises(SystemExit):
                build(report, "1.21.1", 3955)


if __name__ == "__main__":
    unittest.main()
