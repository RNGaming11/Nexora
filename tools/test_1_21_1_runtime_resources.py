import json
import tempfile
import unittest
import zipfile
from pathlib import Path

from tools.build_1_21_1_runtime_resources import build


class RuntimeResources1211Tests(unittest.TestCase):
    def _jar(self, path: Path, block_tag: str = "minecraft:stone") -> None:
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr("assets/minecraft/models/block/stone.json", '{"textures":{}}')
            archive.writestr(
                "data/minecraft/tags/block/blocks_motion.json",
                json.dumps({"values": [block_tag]}),
            )
            archive.writestr("data/minecraft/worldgen/biome/plains.json", '{"temperature":0.8}')

    def _reports(self, root: Path) -> Path:
        reports = root / "reports"
        reports.mkdir()
        (reports / "registries.json").write_text(json.dumps({
            "minecraft:item": {"entries": {
                "minecraft:air": {"protocol_id": 0},
                "minecraft:stone": {"protocol_id": 1},
            }},
            "minecraft:entity_type": {"entries": {
                "minecraft:player": {"protocol_id": 0},
                "minecraft:pig": {"protocol_id": 1},
                "minecraft:cow": {"protocol_id": 2},
            }}
        }), encoding="utf-8")
        (reports / "commands.json").write_text('{"type":"root"}', encoding="utf-8")
        (reports / "blocks.json").write_text(json.dumps({
            "minecraft:air": {"states": [{"id": 0, "default": True}]},
            "minecraft:stone": {"states": [{"id": 1, "default": True}]},
            "minecraft:oak_log": {"states": [
                {"id": 2, "default": True, "properties": {"axis": "y"}},
                {"id": 3, "properties": {"axis": "x"}},
                {"id": 4, "properties": {"axis": "z"}},
            ]},
        }), encoding="utf-8")
        components = reports / "minecraft" / "components" / "item"
        components.mkdir(parents=True)
        (components / "stone.json").write_text(
            '{"components":{"minecraft:max_stack_size":64}}', encoding="utf-8"
        )
        biomes = reports / "biome_parameters" / "minecraft"
        biomes.mkdir(parents=True)
        (biomes / "plains.json").write_text('{"temperature":0.8}', encoding="utf-8")
        return reports

    def _heightmap(self) -> dict:
        return {
            "data_version": 3955,
            "states": 5,
            "motion": [False, True, True, True, True],
            "no_leaves": [False, True, True, True, True],
        }

    def _gameplay(self, root: Path) -> Path:
        path = root / "gameplay-facts.json"
        path.write_text(
            json.dumps({"states": [{"id": index} for index in range(5)]}),
            encoding="utf-8",
        )
        return path

    def _behaviours(self, root: Path) -> Path:
        path = root / "block-behaviours.json"
        path.write_text(
            json.dumps({
                "blocks": [
                    {
                        "name": name,
                        "class": [],
                        "interfaces": [],
                        "fields": {},
                        "dynamic_shape": False,
                        "item": name,
                    }
                    for name in ("minecraft:air", "minecraft:stone", "minecraft:oak_log")
                ],
                "states": [[0, 0, 0, 0, False, 0, 0, False, "HARP"] for _ in range(5)],
                "items": {},
                "fluids": [],
                "fire": {},
            }),
            encoding="utf-8",
        )
        return path

    def _entity_types(self, root: Path) -> Path:
        path = root / "entity-types.json"
        path.write_text(
            json.dumps({
                name: {
                    "width": 0.6,
                    "height": 1.8,
                    "eye_height": 1.62,
                    "category": "creature",
                    "fire_immune": False,
                    "attributes": {},
                }
                for name in ("minecraft:player", "minecraft:pig", "minecraft:cow")
            }),
            encoding="utf-8",
        )
        return path

    def test_builds_bundle_with_inventory_data_and_base_assets(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            client, server = root / "client.jar", root / "server-game.jar"
            output = root / "runtime-resources"
            self._jar(client)
            self._jar(server)
            reports = self._reports(root)
            manifest = build(client, server, reports, output, self._heightmap(), self._gameplay(root), self._behaviours(root), self._entity_types(root))
            self.assertEqual(
                (manifest["minecraft_version"], manifest["data_version"], manifest["protocol_version"]),
                ("1.21.1", 3955, 767),
            )
            for name in (
                "assets/minecraft/models/block/stone.json",
                "data/minecraft/tags/block/blocks_motion.json",
                "builtin-registry-ids.json", "registries.json", "commands.json",
                "facts.json", "heightmap-predicates.json", "gameplay-facts.json", "block-behaviours.json", "entity-types.json", "model-states.json", "items.json",
                "biome-parameters/plains.json", "horizon-resources.json",
            ):
                self.assertTrue((output / name).is_file(), name)
            self.assertFalse((output / "client.jar").exists())
            self.assertFalse((output / "server-game.jar").exists())
            model_states = json.loads((output / "model-states.json").read_text())
            self.assertEqual(
                model_states["minecraft:oak_log"]["schema"],
                {"axis": {"named": ["y", "x", "z"]}},
            )
            rebuilt = build(client, server, reports, output, self._heightmap(), self._gameplay(root), self._behaviours(root), self._entity_types(root))
            self.assertEqual(rebuilt["client_jar_sha1"], manifest["client_jar_sha1"])

    def test_accepts_target_version_without_later_item_and_biome_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            client, server = root / "client.jar", root / "server-game.jar"
            output = root / "runtime-resources"
            self._jar(client)
            self._jar(server)
            reports = self._reports(root)
            (reports / "minecraft" / "components" / "item" / "stone.json").unlink()
            (reports / "minecraft" / "components" / "item").rmdir()
            (reports / "minecraft" / "components").rmdir()
            (reports / "biome_parameters" / "minecraft" / "plains.json").unlink()
            (reports / "biome_parameters" / "minecraft").rmdir()
            (reports / "biome_parameters").rmdir()
            (reports / "items.json").write_text('{"minecraft:stone":{"max_stack_size":64}}', encoding="utf-8")

            manifest = build(client, server, reports, output, self._heightmap(), self._gameplay(root), self._behaviours(root), self._entity_types(root))
            self.assertEqual(manifest["item_component_item_count"], 0)
            self.assertEqual(manifest["biome_parameter_file_count"], 0)
            self.assertEqual(manifest["gameplay_fact_state_count"], 5)
            self.assertEqual(manifest["block_behaviour_block_count"], 3)
            self.assertEqual(manifest["block_behaviour_state_count"], 5)
            self.assertEqual(manifest["entity_type_count"], 3)
            self.assertTrue((output / "gameplay-facts.json").is_file())
            self.assertTrue((output / "block-behaviours.json").is_file())
            self.assertTrue((output / "entity-types.json").is_file())
            self.assertTrue((output / "items-report.json").is_file())
            self.assertFalse((output / "items.json").exists())
            self.assertTrue((output / "data/minecraft/worldgen/biome/plains.json").is_file())


    def test_rejects_client_server_block_tag_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            client, server = root / "client.jar", root / "server-game.jar"
            self._jar(client)
            self._jar(server, block_tag="minecraft:dirt")
            reports = self._reports(root)
            with self.assertRaisesRegex(ValueError, "block tag resources disagree"):
                build(
                    client,
                    server,
                    reports,
                    root / "out",
                    self._heightmap(),
                    self._gameplay(root),
                    self._behaviours(root),
                    self._entity_types(root),
                )


if __name__ == "__main__":
    unittest.main()
