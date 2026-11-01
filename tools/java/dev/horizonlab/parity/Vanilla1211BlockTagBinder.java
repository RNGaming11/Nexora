package dev.horizonlab.parity;

import com.google.gson.JsonParser;
import com.mojang.serialization.JsonOps;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.zip.ZipFile;
import net.minecraft.core.Holder;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.tags.TagFile;
import net.minecraft.tags.TagKey;
import net.minecraft.tags.TagLoader;
import net.minecraft.world.level.block.Block;

public final class Vanilla1211BlockTagBinder {
    private Vanilla1211BlockTagBinder() {}

    @SuppressWarnings("unchecked")
    public static void bind(ZipFile jar) throws Exception {
        var registry = BuiltInRegistries.BLOCK;
        var loader = new TagLoader<Holder<Block>>(
            (id, required) -> registry.get(id).map(value -> (Holder<Block>) value),
            "tags/block"
        );
        var definitions = new HashMap<ResourceLocation, List<TagLoader.EntryWithSource>>();
        var prefix = "data/minecraft/tags/block/";
        var entries = jar.entries();
        while (entries.hasMoreElements()) {
            var entry = entries.nextElement();
            var name = entry.getName();
            if (!name.startsWith(prefix) || !name.endsWith(".json")) {
                continue;
            }
            var text = new String(jar.getInputStream(entry).readAllBytes(), StandardCharsets.UTF_8);
            var file = TagFile.CODEC.parse(JsonOps.INSTANCE, JsonParser.parseString(text)).getOrThrow();
            var list = new ArrayList<TagLoader.EntryWithSource>();
            for (var value : file.entries()) {
                list.add(new TagLoader.EntryWithSource(value, "vanilla"));
            }
            definitions.put(
                ResourceLocation.parse(
                    "minecraft:" + name.substring(prefix.length(), name.length() - 5)
                ),
                list
            );
        }
        var tags = new HashMap<TagKey<Block>, List<Holder<Block>>>();
        loader.build(definitions).forEach(
            (id, values) -> tags.put(TagKey.create(Registries.BLOCK, id), List.copyOf(values))
        );
        if (tags.size() != definitions.size()) {
            throw new IllegalStateException("Incomplete 1.21.1 block tag binding");
        }
        registry.prepareTagReload(new TagLoader.LoadResult<>(Registries.BLOCK, tags)).apply();
    }
}
