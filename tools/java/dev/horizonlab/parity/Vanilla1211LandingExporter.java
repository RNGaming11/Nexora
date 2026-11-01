package dev.horizonlab.parity;

import com.google.gson.JsonObject;
import com.google.gson.Gson;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Set;
import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.sounds.SoundType;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.FarmlandBlock;
import net.minecraft.world.level.block.HayBlock;
import net.minecraft.world.level.block.HoneyBlock;
import net.minecraft.world.level.block.SlimeBlock;
import net.minecraft.world.level.block.PowderSnowBlock;
import net.minecraft.world.level.block.PointedDripstoneBlock;
import net.minecraft.world.level.block.TurtleEggBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;

public final class Vanilla1211LandingExporter {
    private static final Set<String> FALL_FAMILIES = Set.of(
        "Block",
        "HayBlock",
        "HoneyBlock",
        "SlimeBlock",
        "PowderSnowBlock",
        "PointedDripstoneBlock",
        "FarmlandBlock",
        "TurtleEggBlock"
    );

    private static String fallOwner(Block block) {
        for (Class<?> type = block.getClass(); type != null && Block.class.isAssignableFrom(type); type = type.getSuperclass()) {
            for (Method method : type.getDeclaredMethods()) {
                if (method.getName().equals("fallOn")) {
                    return type.getSimpleName();
                }
            }
            if (type == Block.class) {
                break;
            }
        }
        return "Block";
    }

    private static Field field(Class<?> owner, String name) throws ReflectiveOperationException {
        Field result = owner.getDeclaredField(name);
        result.setAccessible(true);
        return result;
    }

    private static JsonObject definitions() throws Exception {
        var result = new JsonObject();
        Field farmlandBase = field(FarmlandBlock.class, "baseBlock");
        Field soundType = field(BlockBehaviour.class, "soundType");

        for (Block block : BuiltInRegistries.BLOCK) {
            String family = fallOwner(block);
            if (!FALL_FAMILIES.contains(family)) {
                throw new IllegalStateException(
                    "Unknown 1.21.1 landing override " + family + " on "
                    + BuiltInRegistries.BLOCK.getKey(block)
                );
            }

            var definition = new JsonObject();
            definition.addProperty("fall", family);
            definition.addProperty("bounce", false);

            if (family.equals("FarmlandBlock")) {
                Block base = (Block) farmlandBase.get(block);
                definition.addProperty(
                    "base",
                    BuiltInRegistries.BLOCK.getKey(base).toString()
                );
            }
            if (family.equals("HoneyBlock")) {
                SoundType sound = (SoundType) soundType.get(block);
                definition.addProperty(
                    "volume",
                    String.format("%08x", Float.floatToIntBits(sound.getVolume()))
                );
                definition.addProperty(
                    "pitch",
                    String.format("%08x", Float.floatToIntBits(sound.getPitch()))
                );
                if (!sound.getFallSound().getLocation().toString().equals("minecraft:block.honey_block.slide")) {
                    throw new IllegalStateException("Unexpected 1.21.1 honey block fall sound");
                }
            }
            result.add(BuiltInRegistries.BLOCK.getKey(block).toString(), definition);
        }
        return result;
    }

    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Path output = Path.of(args[0]);
        Files.createDirectories(output.getParent());
        Files.writeString(output, new Gson().toJson(definitions()), StandardCharsets.UTF_8);
        System.out.println(
            "HORIZON_LANDING_RESULT {\"blocks\":" + BuiltInRegistries.BLOCK.size()
            + ",\"data_version\":" + SharedConstants.getCurrentVersion().dataVersion().version() + "}"
        );
    }
}
