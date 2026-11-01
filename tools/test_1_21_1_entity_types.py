import json
import tempfile
import unittest
from pathlib import Path

from tools.export_1_21_1_entity_types import validate


class EntityTypes1211Tests(unittest.TestCase):
    def _files(self, root: Path) -> tuple[Path, Path]:
        report = root / "registries.json"
        output = root / "entity-types.json"
        report.write_text(
            json.dumps({
                "minecraft:entity_type": {
                    "entries": {
                        "minecraft:pig": {"protocol_id": 0},
                        "minecraft:cow": {"protocol_id": 1},
                    }
                }
            }),
            encoding="utf-8",
        )
        output.write_text(
            json.dumps({
                "minecraft:pig": {
                    "width": 0.9,
                    "height": 0.9,
                    "eye_height": 0.765,
                    "category": "creature",
                    "fire_immune": False,
                    "attributes": {"minecraft:max_health": 10.0},
                },
                "minecraft:cow": {
                    "width": 0.9,
                    "height": 1.4,
                    "eye_height": 1.3,
                    "category": "creature",
                    "fire_immune": False,
                },
            }),
            encoding="utf-8",
        )
        return output, report

    def test_accepts_facts_for_every_registry_entry(self):
        with tempfile.TemporaryDirectory() as directory:
            output, report = self._files(Path(directory))
            result = validate(output, report)
            self.assertEqual(result["entity_types"], 2)
            self.assertGreater(result["bytes"], 0)

    def test_rejects_missing_or_malformed_entity_facts(self):
        with tempfile.TemporaryDirectory() as directory:
            output, report = self._files(Path(directory))
            payload = json.loads(output.read_text(encoding="utf-8"))
            payload.pop("minecraft:cow")
            output.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "do not match"):
                validate(output, report)

            payload["minecraft:cow"] = {
                "width": "wide",
                "height": 1.4,
                "eye_height": 1.3,
                "category": "creature",
                "fire_immune": False,
            }
            output.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "invalid width"):
                validate(output, report)


if __name__ == "__main__":
    unittest.main()
