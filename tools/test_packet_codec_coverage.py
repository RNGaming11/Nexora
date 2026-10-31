import tempfile
import unittest
from pathlib import Path

from tools.audit_packet_codec_coverage import audit, read_ids


class PacketCodecCoverageTests(unittest.TestCase):
    def test_packet_ids_are_read_per_state_and_direction(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "ids.rs"
            path.write_text(
                "pub mod login {\n"
                "    pub mod clientbound {\n"
                "        pub const HELLO: i32 = 1;\n"
                "    }\n"
                "}\n",
                encoding="utf-8",
            )
            self.assertEqual(read_ids(path), {("login", "clientbound"): {"HELLO": 1}})

    def test_isolated_1211_rust_profile_matches_pinned_target(self):
        import json
        import re

        root = Path(__file__).resolve().parents[1]
        target = json.loads((root / "ports/1.21.1/target.json").read_text(encoding="utf-8"))
        source = (root / "ports/1.21.1/version.rs").read_text(encoding="utf-8")
        facts = target["minecraft"]
        expected = {
            "VERSION_ID": facts["id"],
            "VANILLA_VERSION": facts["name"],
            "RELEASE_TIME": facts["release_time"],
            "ASSET_INDEX": int(facts["asset_index"]),
            "PROTOCOL_VERSION": facts["protocol_version"],
            "DATA_VERSION": facts["data_version"],
            "JAVA_VERSION": facts["java_major"],
        }
        for constant, value in expected.items():
            match = re.search(rf"pub const {constant}: [^=]+ = ([^;]+);", source)
            self.assertIsNotNone(match, constant)
            literal = match.group(1).strip()
            if isinstance(value, str):
                self.assertEqual(json.loads(literal), value, constant)
            else:
                self.assertEqual(int(literal), value, constant)
        self.assertEqual(
            tuple(map(int, re.search(r"RESOURCE_PACK_FORMAT: \(i64, i64\) = \((\d+), (\d+)\)", source).groups())),
            (facts["resource_pack_format"], 0),
        )
        self.assertEqual(
            tuple(map(int, re.search(r"DATA_PACK_FORMAT: \(i64, i64\) = \((\d+), (\d+)\)", source).groups())),
            (facts["data_pack_format"], 0),
        )

    def test_recipe_serializer_ids_match_the_official_1211_registry(self):
        import json
        import re

        root = Path(__file__).resolve().parents[1]
        inventory = json.loads(
            (root / "ports/1.21.1/recipe-serializers.json").read_text(encoding="utf-8")
        )
        source = (
            root / "crates/horizon-protocol/src/packets/v1_21_1/recipes.rs"
        ).read_text(encoding="utf-8")
        match = re.search(r"pub const SERIALIZERS: &\[&str\] = &\[(.*?)\];", source, re.S)
        self.assertIsNotNone(match)
        rust_names = re.findall(r'"([^"]+)"', match.group(1))
        self.assertEqual(inventory["registry"], "minecraft:recipe_serializer")
        self.assertEqual(inventory["minecraft_version"], "1.21.1")
        self.assertEqual(inventory["data_version"], 3955)
        self.assertEqual(rust_names, inventory["serializers"])
        self.assertEqual(len(rust_names), 23)

    def test_data_component_ids_match_the_official_1211_registry(self):
        import json
        import re

        root = Path(__file__).resolve().parents[1]
        inventory = json.loads(
            (root / "ports/1.21.1/data-component-ids.json").read_text(encoding="utf-8")
        )
        source = (
            root / "ports/1.21.1/data_component_ids.rs"
        ).read_text(encoding="utf-8")
        match = re.search(r"pub const NAMES: &\[&str\] = &\[(.*?)\];", source, re.S)
        self.assertIsNotNone(match)
        rust_names = re.findall(r'"([^"]+)"', match.group(1))
        self.assertEqual(inventory["registry"], "minecraft:data_component_type")
        self.assertEqual(inventory["minecraft_version"], "1.21.1")
        self.assertEqual(inventory["data_version"], 3955)
        self.assertEqual(rust_names, inventory["components"])
        self.assertEqual(len(rust_names), 57)

    def test_same_name_does_not_hide_packet_id_mismatch(self):
        target = {("play", "clientbound"): {"COMMANDS": 17, "KEEP_ALIVE": 38}}
        codecs = {
            ("play", "clientbound"): {
                "COMMANDS": {"variant": "Commands", "type": "Commands", "version": "26.3"},
                "KEEP_ALIVE": {"variant": "KeepAlive", "type": "KeepAlive", "version": "1.21.1"},
            }
        }
        current = {("play", "clientbound"): {"COMMANDS": 16, "KEEP_ALIVE": 45}}
        result = audit(target, codecs, current)
        state = result["states"]["play/clientbound"]
        self.assertEqual(result["candidate_codec_count"], 2)
        self.assertEqual(result["id_aligned_reuse_candidate_count"], 0)
        self.assertEqual(result["version_specific_codec_count"], 1)
        self.assertEqual(state["id_mismatches"][0]["constant"], "COMMANDS")
        self.assertEqual(state["id_mismatches"][0]["current_id"], 16)
        self.assertEqual(state["version_specific_codecs"][0]["constant"], "KEEP_ALIVE")
        self.assertEqual(result["payload_compatibility_verified_count"], 0)


if __name__ == "__main__":
    unittest.main()
