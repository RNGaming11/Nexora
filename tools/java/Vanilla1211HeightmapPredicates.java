import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Predicate;

public final class Vanilla1211HeightmapPredicates {
    private Vanilla1211HeightmapPredicates() {
    }

    private static Class<?> load(String name) throws ClassNotFoundException {
        return Class.forName(name, true, Thread.currentThread().getContextClassLoader());
    }

    private static Method method(Class<?> owner, String name) throws NoSuchMethodException {
        for (Method method : owner.getDeclaredMethods()) {
            if (method.getName().equals(name) && method.getParameterCount() == 0) {
                method.setAccessible(true);
                return method;
            }
        }
        throw new NoSuchMethodException(owner.getName() + "." + name + "()");
    }

    private static void invokeStatic(String owner, String name) throws Exception {
        Method method = method(load(owner), name);
        method.invoke(null);
    }

    private static Field field(Class<?> owner, String name) throws NoSuchFieldException {
        Field field = owner.getDeclaredField(name);
        field.setAccessible(true);
        return field;
    }

    private static void appendArray(StringBuilder json, List<Boolean> values) {
        json.append('[');
        for (int i = 0; i < values.size(); i++) {
            if (i != 0) {
                json.append(',');
            }
            json.append(values.get(i));
        }
        json.append(']');
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 10) {
            throw new IllegalArgumentException(
                "expected SharedConstants class/method, Bootstrap class/method, Block class/field, "
                    + "Heightmap.Types class/key method/predicate method, and output path"
            );
        }

        invokeStatic(args[0], args[1]);
        invokeStatic(args[2], args[3]);

        Class<?> blockClass = load(args[4]);
        Object states = field(blockClass, args[5]).get(null);
        if (!(states instanceof Iterable<?> iterable)) {
            throw new IllegalStateException("Block.BLOCK_STATE_REGISTRY is not iterable");
        }

        Class<?> typesClass = load(args[6]);
        Method keyMethod = method(typesClass, args[7]);
        Method predicateMethod = method(typesClass, args[8]);
        Predicate<Object> oceanFloor = null;
        Predicate<Object> motionNoLeaves = null;

        Object[] values = typesClass.getEnumConstants();
        if (values == null) {
            throw new IllegalStateException("Heightmap.Types is not an enum");
        }
        for (Object value : values) {
            String key = (String) keyMethod.invoke(value);
            Object predicate = predicateMethod.invoke(value);
            if (!(predicate instanceof Predicate<?>)) {
                throw new IllegalStateException("Heightmap.Types predicate is not a Predicate: " + key);
            }
            @SuppressWarnings("unchecked")
            Predicate<Object> typed = (Predicate<Object>) predicate;
            if ("ocean_floor".equalsIgnoreCase(key)) {
                oceanFloor = typed;
            } else if ("motion_blocking_no_leaves".equalsIgnoreCase(key)) {
                motionNoLeaves = typed;
            }
        }
        if (oceanFloor == null || motionNoLeaves == null) {
            throw new IllegalStateException(
                "Minecraft 1.21.1 heightmap predicates were not found; inspect the target serialization keys"
            );
        }

        List<Boolean> motion = new ArrayList<>();
        List<Boolean> noLeaves = new ArrayList<>();
        for (Object state : iterable) {
            boolean blocksMotion = oceanFloor.test(state);
            motion.add(blocksMotion);
            noLeaves.add(blocksMotion && motionNoLeaves.test(state));
        }
        if (motion.isEmpty()) {
            throw new IllegalStateException("Minecraft registered no block states");
        }

        StringBuilder json = new StringBuilder(motion.size() * 12);
        json.append("{\"data_version\":3955,\"states\":").append(motion.size()).append(",\"motion\":");
        appendArray(json, motion);
        json.append(",\"no_leaves\":");
        appendArray(json, noLeaves);
        json.append("}\n");
        Path output = Path.of(args[9]);
        Files.createDirectories(output.toAbsolutePath().getParent());
        Files.writeString(output, json, StandardCharsets.UTF_8);
        System.out.println("Exported 1.21.1 heightmap predicates for " + motion.size() + " block states");
    }
}
