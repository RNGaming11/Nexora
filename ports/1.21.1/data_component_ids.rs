//! Generated protocol-767 data component registry order from Mojang's 1.21.1 report.

pub const NAMES: &[&str] = &[
    "minecraft:custom_data",
    "minecraft:max_stack_size",
    "minecraft:max_damage",
    "minecraft:damage",
    "minecraft:unbreakable",
    "minecraft:custom_name",
    "minecraft:item_name",
    "minecraft:lore",
    "minecraft:rarity",
    "minecraft:enchantments",
    "minecraft:can_place_on",
    "minecraft:can_break",
    "minecraft:attribute_modifiers",
    "minecraft:custom_model_data",
    "minecraft:hide_additional_tooltip",
    "minecraft:hide_tooltip",
    "minecraft:repair_cost",
    "minecraft:creative_slot_lock",
    "minecraft:enchantment_glint_override",
    "minecraft:intangible_projectile",
    "minecraft:food",
    "minecraft:fire_resistant",
    "minecraft:tool",
    "minecraft:stored_enchantments",
    "minecraft:dyed_color",
    "minecraft:map_color",
    "minecraft:map_id",
    "minecraft:map_decorations",
    "minecraft:map_post_processing",
    "minecraft:charged_projectiles",
    "minecraft:bundle_contents",
    "minecraft:potion_contents",
    "minecraft:suspicious_stew_effects",
    "minecraft:writable_book_content",
    "minecraft:written_book_content",
    "minecraft:trim",
    "minecraft:debug_stick_state",
    "minecraft:entity_data",
    "minecraft:bucket_entity_data",
    "minecraft:block_entity_data",
    "minecraft:instrument",
    "minecraft:ominous_bottle_amplifier",
    "minecraft:jukebox_playable",
    "minecraft:recipes",
    "minecraft:lodestone_tracker",
    "minecraft:firework_explosion",
    "minecraft:fireworks",
    "minecraft:profile",
    "minecraft:note_block_sound",
    "minecraft:banner_patterns",
    "minecraft:base_color",
    "minecraft:pot_decorations",
    "minecraft:container",
    "minecraft:block_state",
    "minecraft:bees",
    "minecraft:lock",
    "minecraft:container_loot",
];

pub fn name(id: i32) -> Option<&'static str> {
    usize::try_from(id).ok().and_then(|index| NAMES.get(index)).copied()
}

pub fn id(name: &str) -> Option<i32> {
    NAMES.iter().position(|candidate| *candidate == name).and_then(|index| i32::try_from(index).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_ids_match_the_pinned_1_21_1_registry_order() {
        assert_eq!(NAMES.len(), 57);
        assert_eq!(name(0), Some("minecraft:custom_data"));
        assert_eq!(name(14), Some("minecraft:hide_additional_tooltip"));
        assert_eq!(name(56), Some("minecraft:container_loot"));
        assert_eq!(name(57), None);
        for (index, component) in NAMES.iter().enumerate() {
            assert_eq!(id(component), Some(index as i32));
        }
    }
}
