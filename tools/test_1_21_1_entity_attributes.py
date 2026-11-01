import json
import tempfile
import unittest
from pathlib import Path

from tools.export_1_21_1_entity_attributes import validate


class EntityAttributes1211Tests(unittest.TestCase):
    def _files(self, root: Path) -> tuple[Path, Path]:
        report = root / "registries.json"
        output = root / "attributes.json"
        report.write_text(
            json.dumps({
                "minecraft:entity_type": {
                    "entries": {
                        "minecraft:player": {"protocol_id": 0},
                        "minecraft:pig": {"protocol_id": 1},
                    }
                },
                "minecraft:attribute": {
                    "entries": {
                        "minecraft:max_health": {"protocol_id": 0},
                        "minecraft:movement_speed": {"protocol_id": 1},
                    }
                },
            }),
            encoding="utf-8",
        )
        output.write_text(
            json.dumps({
                "data_version": 3955,
                "attributes": [
                    {
                        "id": "minecraft:max_health",
                        "default": "403e000000000000",
                        "syncable": True,
                        "min": "0000000000000000",
                        "max": "4059000000000000",
                    },
                    {
                        "id": "minecraft:movement_speed",
                        "default": "3f847ae147ae147b",
                        "syncable": True,
                        "min": "0000000000000000",
                        "max": "3ff0000000000000",
                    },
                ],
                "entities": {
                    "minecraft:player": {
                        "minecraft:max_health": "403e000000000000",
                        "minecraft:movement_speed": "3f847ae147ae147b",
                    },
                    "minecraft:pig": {
                        "minecraft:max_health": "4034000000000000",
                    },
                },
            }),
            encoding="utf-8",
        )
        return output, report

    def test_accepts_complete_registered_attribute_catalog(self):
        with tempfile.TemporaryDirectory() as directory:
            output, report = self._files(Path(directory))
            summary = validate(output, report)
            self.assertEqual(summary["attributes"], 2)
            self.assertEqual(summary["entity_suppliers"], 2)

    def test_rejects_missing_attribute_or_invalid_supplier(self):
        with tempfile.TemporaryDirectory() as directory:
            output, report = self._files(Path(directory))
            payload = json.loads(output.read_text(encoding="utf-8"))
            payload["attributes"].pop()
            output.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "missing registry entries"):
                validate(output, report)

            payload["attributes"].append({
                "id": "minecraft:movement_speed",
                "default": "invalid",
                "syncable": True,
            })
            output.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "invalid default"):
                validate(output, report)


if __name__ == "__main__":
    unittest.main()
