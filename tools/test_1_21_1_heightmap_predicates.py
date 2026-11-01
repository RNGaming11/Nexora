import tempfile
import unittest
from pathlib import Path

from tools.generate_1_21_1_heightmap_predicates import runtime_class, runtime_member
from tools.proguard import ProguardMappings


class HeightmapPredicateMappingTests(unittest.TestCase):
    def test_resolves_target_classes_and_members_from_official_proguard_names(self):
        mapping_text = """\
net.minecraft.world.level.block.Block -> a:
    net.minecraft.core.IdMapper BLOCK_STATE_REGISTRY -> b
net.minecraft.world.level.levelgen.Heightmap$Types -> c:
    java.lang.String getSerializationKey() -> d
    java.util.function.Predicate isOpaque() -> e
net.minecraft.SharedConstants -> f:
    void tryDetectVersion() -> g
net.minecraft.server.Bootstrap -> h:
    void bootStrap() -> i
"""
        with tempfile.TemporaryDirectory() as directory:
            mapping_path = Path(directory) / "server.txt"
            mapping_path.write_text(mapping_text, encoding="utf-8")
            mapping = ProguardMappings.read(mapping_path)

            self.assertEqual(
                runtime_class(mapping, "net.minecraft.world.level.block.Block"),
                "a",
            )
            self.assertEqual(
                runtime_class(mapping, "net.minecraft.world.level.levelgen.Heightmap$Types"),
                "c",
            )
            self.assertEqual(
                runtime_member(
                    mapping,
                    "net.minecraft.world.level.block.Block",
                    "BLOCK_STATE_REGISTRY",
                    "field",
                ),
                "b",
            )
            self.assertEqual(
                runtime_member(
                    mapping,
                    "net.minecraft.world.level.levelgen.Heightmap$Types",
                    "getSerializationKey",
                    "method",
                    "()Ljava/lang/String;",
                ),
                "d",
            )
            self.assertEqual(
                runtime_member(
                    mapping,
                    "net.minecraft.world.level.levelgen.Heightmap$Types",
                    "isOpaque",
                    "method",
                    "()Ljava/util/function/Predicate;",
                ),
                "e",
            )
            self.assertEqual(
                runtime_member(
                    mapping,
                    "net.minecraft.SharedConstants",
                    "tryDetectVersion",
                    "method",
                    "()V",
                ),
                "g",
            )
            self.assertEqual(
                runtime_member(
                    mapping,
                    "net.minecraft.server.Bootstrap",
                    "bootStrap",
                    "method",
                    "()V",
                ),
                "i",
            )

    def test_unmapped_members_keep_their_source_name(self):
        mapping_text = """\
net.minecraft.world.level.block.Block -> a:
"""
        with tempfile.TemporaryDirectory() as directory:
            mapping_path = Path(directory) / "server.txt"
            mapping_path.write_text(mapping_text, encoding="utf-8")
            mapping = ProguardMappings.read(mapping_path)
            self.assertEqual(
                runtime_member(
                    mapping,
                    "net.minecraft.world.level.block.Block",
                    "BLOCK_STATE_REGISTRY",
                    "field",
                ),
                "BLOCK_STATE_REGISTRY",
            )


if __name__ == "__main__":
    unittest.main()
