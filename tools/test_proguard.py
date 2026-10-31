import tempfile
import unittest
from pathlib import Path

from tools.proguard import ProguardMappings


class ProguardMappingTests(unittest.TestCase):
    def parse(self, contents):
        handle = tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False)
        try:
            handle.write(contents)
            handle.close()
            return ProguardMappings.read(handle.name)
        finally:
            Path(handle.name).unlink(missing_ok=True)

    def test_resolves_types_declared_later_and_overloads(self):
        mappings = self.parse("""# comment
net.minecraft.Example -> a:
    void use(net.minecraft.Later) -> b
    void use(int) -> b
    net.minecraft.Later value -> c
net.minecraft.Later -> b:
""")
        self.assertEqual(mappings.class_name("a"), "net/minecraft/Example")
        self.assertEqual(mappings.method_name("a", "b", "(Lb;)V"), "use")
        self.assertEqual(mappings.method_name("a", "b", "(I)V"), "use")
        self.assertEqual(mappings.field_name("a", "c", "Lb;"), "value")

    def test_handles_line_ranges_arrays_and_constructor(self):
        mappings = self.parse("""net.minecraft.Example -> a:
    12:14:void <init>(java.lang.String[]):20:22 -> <init>
    1:2:java.lang.String[] names():3:4 -> c
""")
        self.assertEqual(mappings.method_name("a", "<init>", "([Ljava/lang/String;)V"), "<init>")
        self.assertEqual(mappings.method_name("a", "c", "()[Ljava/lang/String;"), "names")


if __name__ == "__main__":
    unittest.main()
