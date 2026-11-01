import tempfile
import unittest
from pathlib import Path

from tools.proguard import ProguardMappings
from tools.remap_proguard_jar import asm_libraries, write_mapping_table


class ProguardJarRemapperTests(unittest.TestCase):
    def test_mapping_table_preserves_class_member_and_descriptor_names(self):
        source = """\
net.minecraft.world.level.block.Block -> a:
    net.minecraft.core.IdMapper BLOCK_STATE_REGISTRY -> b
    void register(net.minecraft.world.level.block.Block) -> c
net.minecraft.world.level.levelgen.Heightmap$Types -> d:
    java.lang.String getSerializationKey() -> e
"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mapping_file = root / "server.txt"
            output = root / "mappings.tsv"
            mapping_file.write_text(source, encoding="utf-8")
            mapping = ProguardMappings.read(mapping_file)
            write_mapping_table(mapping, output)

            rows = output.read_text(encoding="utf-8").splitlines()
            self.assertIn("C\ta\tnet/minecraft/world/level/block/Block", rows)
            self.assertIn("F\ta\tb\tLnet/minecraft/core/IdMapper;\tBLOCK_STATE_REGISTRY", rows)
            self.assertTrue(
                any(row.startswith("M\ta\tc\t(La;)V\tregister") for row in rows)
            )
            self.assertIn("C\td\tnet/minecraft/world/level/levelgen/Heightmap$Types", rows)

    def test_selects_asm_core_and_commons_from_bundled_server_libraries(self):
        libraries = [
            Path("cache/libraries/com/google/guava/guava/33.0/guava-33.0.jar"),
            Path("cache/libraries/org/ow2/asm/asm/9.6/asm-9.6.jar"),
            Path("cache/libraries/org/ow2/asm/asm-commons/9.6/asm-commons-9.6.jar"),
        ]
        self.assertEqual(asm_libraries(libraries), libraries[1:])

    def test_rejects_missing_asm_commons(self):
        with self.assertRaisesRegex(ValueError, "ASM and asm-commons"):
            asm_libraries([Path("cache/org/ow2/asm/asm/9.6/asm-9.6.jar")])


if __name__ == "__main__":
    unittest.main()
