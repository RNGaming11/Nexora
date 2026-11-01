import json
import tempfile
import unittest
from pathlib import Path

from tools.generate_1_21_1_block_facts import build


class BlockFacts1211Tests(unittest.TestCase):
    def test_state_ids_are_sorted_and_defaults_and_fluid_flags_are_emitted(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "blocks.json"
            report.write_text(
                json.dumps({
                    "minecraft:stone": {
                        "states": [{"id": 2, "default": True}]
                    },
                    "minecraft:water": {
                        "states": [
                            {"id": 0, "default": True},
                            {"id": 1, "properties": {"level": "1"}},
                        ]
                    },
                    "minecraft:oak_slab": {
                        "states": [
                            {"id": 3, "default": True, "properties": {"waterlogged": "true"}}
                        ]
                    },
                }),
                encoding="utf-8",
            )
            facts = build(report)
            self.assertEqual(len(facts["states"]), 4)
            self.assertEqual(facts["defaults"]["minecraft:water"], 0)
            self.assertEqual(facts["states"][0][0], "minecraft:water")
            self.assertTrue(facts["states"][0][2])
            self.assertFalse(facts["states"][2][2])
            self.assertTrue(facts["states"][3][2])
            self.assertEqual(
                facts["states"][3][4],
                {"id": "minecraft:oak_slab", "properties": {"waterlogged": "true"}},
            )

    def test_exact_heightmap_predicates_are_appended_by_state_id(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "blocks.json"
            report.write_text(
                json.dumps({
                    "minecraft:air": {"states": [{"id": 0, "default": True}]},
                    "minecraft:stone": {"states": [{"id": 1, "default": True}]},
                }),
                encoding="utf-8",
            )
            predicates = {
                "data_version": 3955,
                "states": 2,
                "motion": [False, True],
                "no_leaves": [False, True],
            }
            facts = build(report, heightmap_predicates=predicates)
            self.assertEqual(facts["states"][0][5:], [False, False])
            self.assertEqual(facts["states"][1][5:], [True, True])

            predicates["states"] = 1
            with self.assertRaises(SystemExit):
                build(report, heightmap_predicates=predicates)

    def test_air_flags_and_one_default_per_block_are_required(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "blocks.json"
            report.write_text(
                json.dumps({
                    "minecraft:air": {
                        "states": [{"id": 0, "default": True}]
                    }
                }),
                encoding="utf-8",
            )
            self.assertTrue(build(report)["states"][0][1])
            report.write_text(
                json.dumps({
                    "minecraft:air": {"states": [{"id": 0}]}
                }),
                encoding="utf-8",
            )
            with self.assertRaises(SystemExit):
                build(report)

    def test_non_contiguous_and_duplicate_state_ids_fail_closed(self):
        for states in [
            [{"id": 0, "default": True}, {"id": 2}],
            [{"id": 0, "default": True}, {"id": 0}],
        ]:
            with self.subTest(states=states), tempfile.TemporaryDirectory() as directory:
                report = Path(directory) / "blocks.json"
                report.write_text(
                    json.dumps({"minecraft:test": {"states": states}}),
                    encoding="utf-8",
                )
                with self.assertRaises(SystemExit):
                    build(report)


if __name__ == "__main__":
    unittest.main()
