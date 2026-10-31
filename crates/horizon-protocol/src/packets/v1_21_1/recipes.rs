//! Protocol-767 vanilla recipe serializer codecs.
//!
//! The serializer IDs are read from the official 1.21.1 built-in registry
//! report. Payload layouts follow the 1.21.1 packet schema. Recipe slots with
//! data-component patches are rejected until their version-specific codecs
//! are implemented; treating them as 26.3 components would corrupt the stream.
use super::{Error, RecipeSerializer, Result};
use crate::buf::{Reader, Writer};

pub const SERIALIZERS: &[&str] = &[
    "minecraft:crafting_shaped",
    "minecraft:crafting_shapeless",
    "minecraft:crafting_special_armordye",
    "minecraft:crafting_special_bookcloning",
    "minecraft:crafting_special_mapcloning",
    "minecraft:crafting_special_mapextending",
    "minecraft:crafting_special_firework_rocket",
    "minecraft:crafting_special_firework_star",
    "minecraft:crafting_special_firework_star_fade",
    "minecraft:crafting_special_tippedarrow",
    "minecraft:crafting_special_bannerduplicate",
    "minecraft:crafting_special_shielddecoration",
    "minecraft:crafting_special_shulkerboxcoloring",
    "minecraft:crafting_special_suspiciousstew",
    "minecraft:crafting_special_repairitem",
    "minecraft:smelting",
    "minecraft:blasting",
    "minecraft:smoking",
    "minecraft:campfire_cooking",
    "minecraft:stonecutting",
    "minecraft:smithing_transform",
    "minecraft:smithing_trim",
    "minecraft:crafting_decorated_pot",
];

pub fn serializer_name(id: i32) -> Result<&'static str> {
    usize::try_from(id)
        .ok()
        .and_then(|index| SERIALIZERS.get(index))
        .copied()
        .ok_or_else(|| Error::new(format!("unknown protocol-767 recipe serializer id {id}")))
}

pub fn serializer_id(name: &str) -> Result<i32> {
    SERIALIZERS
        .iter()
        .position(|candidate| *candidate == name)
        .and_then(|id| i32::try_from(id).ok())
        .ok_or_else(|| Error::new(format!("unknown protocol-767 recipe serializer {name}")))
}

/// A recipe slot with no data-component patch. This is sufficient for the
/// vanilla 1.21.1 recipe payloads represented by the standard serializers.
/// Component-bearing slots fail closed rather than using the 26.3 component
/// registry, whose IDs and codecs differ.
fn read_slot(r: &mut Reader) -> Result<()> {
    let count = r.varint()?;
    if count < 0 {
        return Err(Error::new("negative item count in 1.21.1 recipe slot"));
    }
    if count == 0 {
        return Ok(());
    }
    let item_id = r.varint()?;
    if item_id < 0 {
        return Err(Error::new("negative item registry id in 1.21.1 recipe slot"));
    }
    let added = r.count(256)?;
    let removed = r.count(256)?;
    if added != 0 || removed != 0 {
        return Err(Error::new(
            "component-bearing 1.21.1 recipe slots are not supported yet",
        ));
    }
    Ok(())
}

fn read_ingredient(r: &mut Reader) -> Result<()> {
    let count = r.count(1024)?;
    for _ in 0..count {
        read_slot(r)?;
    }
    Ok(())
}

fn read_ingredients(r: &mut Reader) -> Result<()> {
    let count = r.count(4096)?;
    for _ in 0..count {
        read_ingredient(r)?;
    }
    Ok(())
}

fn skip_recipe(serializer: &str, r: &mut Reader) -> Result<()> {
    match serializer {
        "minecraft:crafting_shaped" => {
            let _group = r.string(32767)?;
            let _category = r.varint()?;
            let width = r.varint()?;
            let height = r.varint()?;
            if !(1..=3).contains(&width) || !(1..=3).contains(&height) {
                return Err(Error::new("invalid 1.21.1 shaped recipe dimensions"));
            }
            for _ in 0..width {
                for _ in 0..height {
                    read_ingredient(r)?;
                }
            }
            read_slot(r)?;
            let _show_notification = r.bool()?;
        }
        "minecraft:crafting_shapeless" => {
            let _group = r.string(32767)?;
            let _category = r.varint()?;
            read_ingredients(r)?;
            read_slot(r)?;
        }
        "minecraft:crafting_special_armordye"
        | "minecraft:crafting_special_bookcloning"
        | "minecraft:crafting_special_mapcloning"
        | "minecraft:crafting_special_mapextending"
        | "minecraft:crafting_special_firework_rocket"
        | "minecraft:crafting_special_firework_star"
        | "minecraft:crafting_special_firework_star_fade"
        | "minecraft:crafting_special_tippedarrow"
        | "minecraft:crafting_special_bannerduplicate"
        | "minecraft:crafting_special_shielddecoration"
        | "minecraft:crafting_special_shulkerboxcoloring"
        | "minecraft:crafting_special_suspiciousstew"
        | "minecraft:crafting_special_repairitem"
        | "minecraft:crafting_decorated_pot" => {
            let _category = r.varint()?;
        }
        "minecraft:smelting"
        | "minecraft:blasting"
        | "minecraft:smoking"
        | "minecraft:campfire_cooking" => {
            let _group = r.string(32767)?;
            let _category = r.varint()?;
            read_ingredient(r)?;
            read_slot(r)?;
            let _experience = r.f32()?;
            let _cooking_time = r.varint()?;
        }
        "minecraft:stonecutting" => {
            let _group = r.string(32767)?;
            read_ingredient(r)?;
            read_slot(r)?;
        }
        "minecraft:smithing_transform" => {
            read_ingredient(r)?;
            read_ingredient(r)?;
            read_ingredient(r)?;
            read_slot(r)?;
        }
        "minecraft:smithing_trim" => {
            read_ingredient(r)?;
            read_ingredient(r)?;
            read_ingredient(r)?;
        }
        other => return Err(Error::new(format!("unsupported 1.21.1 recipe serializer {other}"))),
    }
    Ok(())
}

/// The standard vanilla recipe serializers for protocol 767. This codec
/// preserves each parsed recipe body byte-for-byte for later re-encoding.
#[derive(Clone, Copy, Debug, Default)]
pub struct VanillaRecipeSerializers;

impl RecipeSerializer for VanillaRecipeSerializers {
    fn read_recipe(&self, serializer: &str, _id: &str, input: &mut Reader) -> Result<Vec<u8>> {
        let start = input.position();
        skip_recipe(serializer, input)?;
        Ok(input.since(start).to_vec())
    }

    fn write_recipe(
        &self,
        serializer: &str,
        _id: &str,
        body: &[u8],
        output: &mut Writer,
    ) -> Result<()> {
        let mut reader = Reader::new(body);
        skip_recipe(serializer, &mut reader)?;
        if reader.remaining() != 0 {
            return Err(Error::new(format!(
                "1.21.1 recipe serializer {serializer} left {} trailing bytes",
                reader.remaining()
            )));
        }
        output.bytes(body);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializer_ids_match_the_1_21_1_registry_order() {
        assert_eq!(SERIALIZERS.len(), 23);
        assert_eq!(serializer_name(0).unwrap(), "minecraft:crafting_shaped");
        assert_eq!(serializer_name(15).unwrap(), "minecraft:smelting");
        assert_eq!(serializer_name(22).unwrap(), "minecraft:crafting_decorated_pot");
        assert!(serializer_name(23).is_err());
        for (id, name) in SERIALIZERS.iter().enumerate() {
            assert_eq!(serializer_id(name).unwrap(), id as i32);
        }
    }

    #[test]
    fn parses_and_reencodes_a_shaped_recipe_body() {
        let mut body = Writer::new();
        body.string("");
        body.varint(0);
        body.varint(1);
        body.varint(1);
        body.varint(1);
        body.varint(1);
        body.varint(1);
        body.varint(0);
        body.varint(0);
        body.varint(0);
        body.bool(true);
        let body = body.data;

        let codec = VanillaRecipeSerializers;
        let mut input = Reader::new(&body);
        let parsed = codec
            .read_recipe("minecraft:crafting_shaped", "minecraft:test", &mut input)
            .unwrap();
        assert_eq!(parsed, body);
        assert_eq!(input.remaining(), 0);

        let mut output = Writer::new();
        codec
            .write_recipe("minecraft:crafting_shaped", "minecraft:test", &parsed, &mut output)
            .unwrap();
        assert_eq!(output.data, body);
    }

    #[test]
    fn every_registered_vanilla_serializer_accepts_its_empty_wire_shape() {
        for serializer in SERIALIZERS {
            let mut body = Writer::new();
            match *serializer {
                "minecraft:crafting_shaped" => {
                    body.string("");
                    body.varint(0);
                    body.varint(1);
                    body.varint(1);
                    body.varint(0);
                    body.varint(0);
                    body.bool(false);
                }
                "minecraft:crafting_shapeless" => {
                    body.string("");
                    body.varint(0);
                    body.varint(0);
                    body.varint(0);
                }
                "minecraft:smelting"
                | "minecraft:blasting"
                | "minecraft:smoking"
                | "minecraft:campfire_cooking" => {
                    body.string("");
                    body.varint(0);
                    body.varint(0);
                    body.varint(0);
                    body.f32(0.0);
                    body.varint(0);
                }
                "minecraft:stonecutting" => {
                    body.string("");
                    body.varint(0);
                    body.varint(0);
                }
                "minecraft:smithing_transform" => {
                    body.varint(0);
                    body.varint(0);
                    body.varint(0);
                    body.varint(0);
                }
                "minecraft:smithing_trim" => {
                    body.varint(0);
                    body.varint(0);
                    body.varint(0);
                }
                _ => body.varint(0),
            }
            let bytes = body.data;
            let mut input = Reader::new(&bytes);
            let codec = VanillaRecipeSerializers;
            let parsed = codec
                .read_recipe(serializer, "minecraft:test", &mut input)
                .unwrap_or_else(|error| panic!("{serializer}: {error}"));
            assert_eq!(parsed, bytes, "{serializer}");
            assert_eq!(input.remaining(), 0, "{serializer}");
            let mut output = Writer::new();
            codec
                .write_recipe(serializer, "minecraft:test", &parsed, &mut output)
                .unwrap_or_else(|error| panic!("{serializer}: {error}"));
            assert_eq!(output.data, bytes, "{serializer}");
        }
    }

    #[test]
    fn parses_special_recipe_category_and_rejects_component_slots() {
        let codec = VanillaRecipeSerializers;
        let mut special = Reader::new(&[2]);
        assert_eq!(
            codec
                .read_recipe(
                    "minecraft:crafting_special_armordye",
                    "minecraft:test",
                    &mut special,
                )
                .unwrap(),
            vec![2]
        );

        let mut component_slot = Reader::new(&[1, 5, 1, 0, 0]);
        assert!(read_slot(&mut component_slot).is_err());
    }

    #[test]
    fn recipe_body_rejects_trailing_bytes_on_write() {
        let codec = VanillaRecipeSerializers;
        let mut output = Writer::new();
        assert!(
            codec
                .write_recipe(
                    "minecraft:crafting_special_armordye",
                    "minecraft:test",
                    &[1, 0],
                    &mut output,
                )
                .is_err()
        );
        assert!(output.data.is_empty());
    }
}
