package dev.horizonlab.parity;
import com.google.gson.*;
import java.io.*;import java.lang.reflect.*;import java.nio.file.*;import java.util.*;
import net.minecraft.core.*;import net.minecraft.core.registries.*;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.world.entity.*;
import net.minecraft.world.entity.ai.attributes.*;
/** Native lazy entity attributes; set membership is observed in registry order. */
public final class Vanilla1211EntityAttributeExporter {
    static String bits(double v){return String.format("%016x",Double.doubleToLongBits(v));}
    static double number(JsonElement v){return Double.longBitsToDouble(Long.parseUnsignedLong(v.getAsString(),16));}
    static Field field(Class<?> c,String n)throws Exception{var f=c.getDeclaredField(n);f.setAccessible(true);return f;}
    static List<Holder<Attribute>> attributes=new ArrayList<>();
    static JsonObject export()throws Exception {
        var root=new JsonObject();root.addProperty("data_version",3955);
        var defs=new JsonArray();
        for(var a:attributes){var v=a.value();var d=new JsonObject();d.addProperty("id",BuiltInRegistries.ATTRIBUTE.getKey(v).toString());d.addProperty("default",bits(v.getDefaultValue()));d.addProperty("syncable",v.isClientSyncable());if(v instanceof RangedAttribute r){d.addProperty("min",bits(r.getMinValue()));d.addProperty("max",bits(r.getMaxValue()));}defs.add(d);}
        root.add("attributes",defs);var entities=new JsonObject();
        for(var e:BuiltInRegistries.ENTITY_TYPE){if(!DefaultAttributes.hasSupplier(e))continue;var supplier=supplier(e);var bases=new JsonObject();for(var a:attributes)if(supplier.hasAttribute(a)){var instance=supplier.createInstance(v->{},a);if(!instance.getModifiers().isEmpty())throw new AssertionError("default modifiers require export support");bases.addProperty(a.getRegisteredName(),bits(supplier.getBaseValue(a)));}entities.add(BuiltInRegistries.ENTITY_TYPE.getKey(e).toString(),bases);}
        root.add("entities",entities);return root;
    }
    @SuppressWarnings("unchecked") static AttributeSupplier supplier(EntityType<?> e){return DefaultAttributes.getSupplier((EntityType<? extends LivingEntity>)e);}
    public static void main(String[] args)throws Exception {
        net.minecraft.SharedConstants.tryDetectVersion();
        net.minecraft.server.Bootstrap.bootStrap();
        for(var a:BuiltInRegistries.ATTRIBUTE)attributes.add(BuiltInRegistries.ATTRIBUTE.wrapAsHolder(a));
        if(args.length!=2||!args[0].equals("export"))throw new IllegalArgumentException("Usage: export OUTPUT");
        Path output=Path.of(args[1]);
        Files.createDirectories(output.toAbsolutePath().getParent());
        Files.writeString(output,export().toString());
    }
}
