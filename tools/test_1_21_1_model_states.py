import json
import tempfile
import unittest
from pathlib import Path

from tools.generate_1_21_1_model_states import build


class ModelStates1211Tests(unittest.TestCase):
    def test_named_integer_and_property_free_blocks(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "blocks.json"
            report.write_text(json.dumps({
                "minecraft:oak_log": {"states": [
                    {"id": 0, "properties": {"axis": "y"}},
                    {"id": 1, "properties": {"axis": "x"}},
                    {"id": 2, "properties": {"axis": "z"}},
                ]},
                "minecraft:age_test": {"states": [
                    {"id": 3, "properties": {"age": "0"}},
                    {"id": 4, "properties": {"age": "3"}},
                    {"id": 5, "properties": {"age": "7"}},
                ]},
                "minecraft:stone": {"states": [{"id": 6}]},
                "minecraft:oak_stairs": {"states": [
                    {"id": 7, "properties": {"facing": "north"}},
                    {"id": 8, "properties": {"facing": "south"}},
                    {"id": 9, "properties": {"facing": "west"}},
                    {"id": 10, "properties": {"facing": "east"}},
                ]},
            }), encoding="utf-8")

            result = build(report)
            self.assertEqual(
                result["minecraft:oak_log"]["schema"],
                {"axis": {"named": ["x", "y", "z"]}},
            )
            self.assertEqual(
                result["minecraft:age_test"]["schema"],
                {"age": {"integer": [0, 7]}},
            )
            self.assertEqual(result["minecraft:stone"]["states"], [{}])
            self.assertEqual(
                result["minecraft:oak_stairs"]["schema"],
                {"facing": {"named": ["north", "south", "west", "east"]}},
            )

    def test_rejects_inconsistent_property_sets(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "blocks.json"
            report.write_text(json.dumps({
                "minecraft:bad": {"states": [
                    {"id": 0, "properties": {"axis": "x"}},
                    {"id": 1, "properties": {"facing": "north"}},
                ]}
            }), encoding="utf-8")
            with self.assertRaises(SystemExit):
                build(report)


if __name__ == "__main__":
    unittest.main()
