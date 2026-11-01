import tempfile
import unittest
from pathlib import Path

from tools.proguard import ProguardMappings
from tools.remap_proguard_jar import JVM_EXPORTS, write_mapping_table


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

    def test_remapper_uses_only_the_jdk_21_internal_asm_modules(self):
        self.assertEqual(len(JVM_EXPORTS), 2)
        self.assertTrue(
            any("jdk.internal.org.objectweb.asm=" in value for value in JVM_EXPORTS)
        )
        self.assertTrue(
            any("jdk.internal.org.objectweb.asm.commons=" in value for value in JVM_EXPORTS)
        )


if __name__ == "__main__":
    unittest.main()
