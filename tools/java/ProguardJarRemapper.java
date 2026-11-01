import java.io.BufferedReader;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import java.util.zip.ZipOutputStream;

import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.commons.ClassRemapper;
import org.objectweb.asm.commons.Remapper;

public final class ProguardJarRemapper {
    private static final class Names extends Remapper {
        private final Map<String, String> classes = new HashMap<>();
        private final Map<String, String> fields = new HashMap<>();
        private final Map<String, String> methods = new HashMap<>();

        Names(Path path) throws IOException {
            try (BufferedReader reader = Files.newBufferedReader(path, StandardCharsets.UTF_8)) {
                String line;
                while ((line = reader.readLine()) != null) {
                    String[] parts = line.split("\\t", -1);
                    if (parts.length == 3 && parts[0].equals("C")) {
                        classes.put(parts[1], parts[2]);
                    } else if (parts.length == 5 && parts[0].equals("F")) {
                        fields.put(key(parts[1], parts[2], parts[3]), parts[4]);
                    } else if (parts.length == 5 && parts[0].equals("M")) {
                        methods.put(key(parts[1], parts[2], parts[3]), parts[4]);
                    } else {
                        throw new IOException("Invalid mapping row: " + line);
                    }
                }
            }
            if (classes.isEmpty()) {
                throw new IOException("Mapping file contains no classes");
            }
        }

        private static String key(String owner, String name, String descriptor) {
            return owner + "|" + name + "|" + descriptor;
        }

        @Override
        public String map(String internalName) {
            return classes.getOrDefault(internalName, internalName);
        }

        @Override
        public String mapFieldName(String owner, String name, String descriptor) {
            return fields.getOrDefault(key(owner, name, descriptor), name);
        }

        @Override
        public String mapMethodName(String owner, String name, String descriptor) {
            return methods.getOrDefault(key(owner, name, descriptor), name);
        }
    }

    private static String unsignedEntryName(String name) {
        String upper = name.toUpperCase(java.util.Locale.ROOT);
        if (!upper.startsWith("META-INF/")) {
            return name;
        }
        String leaf = upper.substring(upper.lastIndexOf('/') + 1);
        if (leaf.startsWith("SIG-")) {
            return null;
        }
        for (String suffix : new String[] {".SF", ".RSA", ".DSA", ".EC"}) {
            if (leaf.endsWith(suffix)) {
                return null;
            }
        }
        return name;
    }

    private static String remapClassEntry(String entryName, Names remapper) {
        String prefix = "";
        String path = entryName;
        if (path.startsWith("META-INF/versions/")) {
            int split = path.indexOf('/', "META-INF/versions/".length());
            if (split >= 0) {
                prefix = path.substring(0, split + 1);
                path = path.substring(split + 1);
            }
        }
        if (!path.endsWith(".class")) {
            return entryName;
        }
        String internalName = path.substring(0, path.length() - ".class".length());
        return prefix + remapper.map(internalName) + ".class";
    }

    private static byte[] remapClass(byte[] input, Names remapper) {
        ClassReader reader = new ClassReader(input);
        ClassWriter writer = new ClassWriter(0);
        reader.accept(new ClassRemapper(writer, remapper), 0);
        return writer.toByteArray();
    }

    private static byte[] remapService(byte[] input, Names remapper) {
        String text = new String(input, StandardCharsets.UTF_8);
        StringBuilder output = new StringBuilder(text.length());
        for (String line : text.split("\\R", -1)) {
            String trimmed = line.trim();
            if (!trimmed.isEmpty() && !trimmed.startsWith("#")) {
                String prefix = line.substring(0, line.indexOf(trimmed));
                String suffix = line.substring(line.indexOf(trimmed) + trimmed.length());
                line = prefix + remapper.map(trimmed.replace('.', '/')).replace('/', '.') + suffix;
            }
            output.append(line).append('\n');
        }
        return output.toString().getBytes(StandardCharsets.UTF_8);
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 3) {
            throw new IllegalArgumentException("usage: ProguardJarRemapper INPUT.jar MAPPINGS.tsv OUTPUT.jar");
        }
        Path input = Path.of(args[0]);
        Path mappings = Path.of(args[1]);
        Path output = Path.of(args[2]);
        Names remapper = new Names(mappings);
        Files.createDirectories(output.toAbsolutePath().getParent());

        Set<String> written = new HashSet<>();
        long classes = 0;
        long resources = 0;
        try (ZipFile source = new ZipFile(input.toFile());
             ZipOutputStream destination = new ZipOutputStream(Files.newOutputStream(output))) {
            var entries = source.entries();
            while (entries.hasMoreElements()) {
                ZipEntry entry = entries.nextElement();
                if (entry.isDirectory()) {
                    continue;
                }
                String originalName = entry.getName();
                String safeName = unsignedEntryName(originalName);
                if (safeName == null) {
                    continue;
                }
                String targetName = remapClassEntry(safeName, remapper);
                if (targetName.startsWith("META-INF/services/")) {
                    String serviceName = targetName.substring("META-INF/services/".length());
                    targetName = "META-INF/services/" + remapper.map(serviceName.replace('.', '/')).replace('/', '.');
                }
                if (!written.add(targetName)) {
                    throw new IOException("Two jar entries remap to the same path: " + targetName);
                }

                byte[] data;
                try (var stream = source.getInputStream(entry)) {
                    if (safeName.endsWith(".class")) {
                        data = remapClass(stream.readAllBytes(), remapper);
                        classes++;
                    } else {
                        data = stream.readAllBytes();
                        if (safeName.startsWith("META-INF/services/")) {
                            data = remapService(data, remapper);
                        }
                        resources++;
                    }
                }

                ZipEntry mapped = new ZipEntry(targetName);
                mapped.setTime(entry.getTime());
                destination.putNextEntry(mapped);
                destination.write(data);
                destination.closeEntry();
            }
        }
        System.out.println(
            "Remapped " + classes + " classes and copied " + resources + " resources from " + input
        );
    }
}
