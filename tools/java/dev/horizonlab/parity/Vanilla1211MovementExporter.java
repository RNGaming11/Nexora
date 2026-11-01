package dev.horizonlab.parity;

import java.io.DataOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.nbt.ListTag;
import net.minecraft.nbt.NbtIo;
import net.minecraft.nbt.NbtOps;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.FenceGateBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;

public final class Vanilla1211MovementExporter {
    private static final String[] FIELDS = {"friction", "speedFactor", "jumpFactor"};

    private static Field field(String name) throws ReflectiveOperationException {
        Field result = BlockBehaviour.class.getDeclaredField(name);
        result.setAccessible(true);
        return result;
    }

    private static void write(CompoundTag tag, Path path) throws IOException {
        try (DataOutputStream output = new DataOutputStream(Files.newOutputStream(path))) {
            NbtIo.write(tag, output);
        }
    }

    private static void export(Path directory) throws Exception {
        var movementFields = new Field[FIELDS.length];
        for (int index = 0; index < FIELDS.length; index++) {
            movementFields[index] = field(FIELDS[index]);
        }

        var states = new ListTag();
        var defaults = new CompoundTag();
        for (var state : Block.BLOCK_STATE_REGISTRY) {
            states.add(BlockState.FULL_CODEC.encodeStart(NbtOps.INSTANCE, state).getOrThrow());
            if (state == state.getBlock().defaultBlockState()) {
                defaults.putInt(
                    BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString(),
                    Block.getId(state)
                );
            }
        }

        var definitions = new CompoundTag();
        for (var block : BuiltInRegistries.BLOCK) {
            var data = new CompoundTag();
            for (int index = 0; index < movementFields.length; index++) {
                data.putFloat(FIELDS[index], movementFields[index].getFloat(block));
            }
            data.putFloat("bounceRestitution", 0.0F);
            data.putFloat("fallDistanceReduction", 0.0F);
            data.putBoolean("fenceGate", block instanceof FenceGateBlock);
            definitions.put(BuiltInRegistries.BLOCK.getKey(block).toString(), data);
        }

        int dataVersion = SharedConstants.getCurrentVersion().dataVersion().version();
        var registry = new CompoundTag();
        registry.putInt("DataVersion", dataVersion);
        registry.put("states", states.copy());
        registry.put("defaults", defaults);

        var movement = new CompoundTag();
        movement.putInt("DataVersion", dataVersion);
        movement.put("states", states);
        movement.put("blocks", definitions);

        write(registry, directory.resolve("block-registry.nbt"));
        write(movement, directory.resolve("movement-properties.nbt"));
        System.out.printf(
            "HORIZON_MOVEMENT_RESULT {\"states\":%d,\"blocks\":%d,\"data_version\":%d}%n",
            Block.BLOCK_STATE_REGISTRY.size(),
            BuiltInRegistries.BLOCK.size(),
            dataVersion
        );
    }

    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Path directory = Path.of(args[0]);
        Files.createDirectories(directory);
        export(directory);
    }
}
