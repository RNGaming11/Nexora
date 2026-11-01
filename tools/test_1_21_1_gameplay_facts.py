import json
import tempfile
import unittest
from pathlib import Path

from tools.export_1_21_1_gameplay_facts import validate


def state(index: int) -> dict:
    return {
        "id": index,
        "emission": 0,
        "dampening": 0,
        "skylight_down": False,
        "shape_occlusion": False,
        "can_occlude": False,
        "solid_render": False,
        "hardness": 1.0,
        "requires_tool": False,
        "replaceable": False,
        "random_ticks": False,
        "block_entity": False,
        "liquid": False,
        "ignited_by_lava": False,
        "push": "NORMAL",
        "render_shape": "MODEL",
        "full_collision": True,
        "friction": 0.6,
        "explosion_resistance": 6.0,
        "item": "minecraft:stone",
        "sound": {
            "volume": 1.0,
            "pitch": 1.0,
            "break": {"id": "minecraft:block.stone.break"},
            "step": {"id": "minecraft:block.stone.step"},
            "place": {"id": "minecraft:block.stone.place"},
            "hit": {"id": "minecraft:block.stone.hit"},
            "fall": {"id": "minecraft:block.stone.fall"},
        },
        "offset": False,
        "outline_offset": False,
        "map_color": 11,
        "outline": [[0, 0, 0, 1, 1, 1]],
        "outline_edges": [[0, 0, 0, 1, 1, 1]],
    }


class GameplayFacts1211Tests(unittest.TestCase):
    def _files(self, root: Path) -> tuple[Path, Path]:
        blocks = root / "blocks.json"
        facts = root / "gameplay-facts.json"
        blocks.write_text(
            json.dumps({
                "minecraft:air": {"states": [{"id": 0, "default": True}]},
                "minecraft:stone": {"states": [{"id": 1, "default": True}]},
            }),
            encoding="utf-8",
        )
        facts.write_text(json.dumps({"states": [state(0), state(1)]}), encoding="utf-8")
        return facts, blocks

    def test_accepts_complete_target_gameplay_fact_table(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            facts, blocks = self._files(root)
            summary = validate(facts, blocks)
            self.assertEqual(summary["states"], 2)
            self.assertGreater(summary["bytes"], 0)

    def test_rejects_missing_fields_and_wrong_registry_count(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            facts, blocks = self._files(root)
            payload = json.loads(facts.read_text(encoding="utf-8"))
            del payload["states"][1]["sound"]["fall"]
            facts.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "sound IDs"):
                validate(facts, blocks)

            payload["states"].pop()
            facts.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "official registry"):
                validate(facts, blocks)


if __name__ == "__main__":
    unittest.main()
