use std::{collections::BTreeSet, env, fs, path::PathBuf, process};

use horizon_client_render::{models::StateModels, sprites::Sprites};
use horizon_protocol::registry::Builtin;
use horizon_world::{
    registry::{BlockRegistry, BlockStateId},
    worldgen::tags::TagDefinitions,
};

fn main() {
    if let Err(error) = validate() {
        eprintln!("Minecraft 1.21.1 runtime resource validation failed: {error}");
        process::exit(1);
    }
}

fn validate() -> Result<(), Box<dyn std::error::Error>> {
    let root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: validate_1_21_1_resources RESOURCES_DIRECTORY")?;

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("horizon-resources.json"))?)?;
    if manifest["minecraft_version"].as_str() != Some("1.21.1")
        || manifest["data_version"].as_i64() != Some(3955)
        || manifest["protocol_version"].as_i64() != Some(767)
    {
        return Err("resource manifest is not pinned to Minecraft 1.21.1".into());
    }

    let inventory_path = Builtin::find(&root).ok_or("built-in registry inventory is missing")?;
    let builtin = Builtin::load(&inventory_path)?;
    for registry in [
        "minecraft:block",
        "minecraft:item",
        "minecraft:fluid",
        "minecraft:entity_type",
    ] {
        if builtin.get(registry).is_none() {
            return Err(format!("required built-in registry is missing: {registry}").into());
        }
    }

    let (registry, _flags) = BlockRegistry::from_facts_json_with_data_version(
        fs::File::open(root.join("facts.json"))?,
        3955,
    )?;
    let mut block_names = BTreeSet::new();
    for index in 0..registry.len() {
        let state = registry
            .state(BlockStateId(index as u32))
            .ok_or("block state registry contains a gap")?;
        block_names.insert(state.name.as_str());
    }

    let tags: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&fs::read(root.join("block-tags.json"))?)?;
    if tags.is_empty() {
        return Err("vanilla block tag bundle is empty".into());
    }
    let mut definitions = TagDefinitions::default();
    for (name, text) in &tags {
        definitions.apply_json(
            name,
            "vanilla",
            text.as_str().ok_or("block tag value is not JSON text")?,
        )?;
    }
    let built =
        definitions.build(|name, _| registry.default_state(name).map(|_| name.to_owned()))?;
    if !built.missing.is_empty() {
        return Err(format!("unresolved vanilla block tags: {:?}", built.missing).into());
    }

    let blocks: Vec<&str> = block_names.into_iter().collect();
    let mut sprites = Sprites::default();
    let _models = StateModels::load(&root, &registry, &blocks, &mut sprites)?;
    sprites.finish();
    if sprites.layers.is_empty() {
        return Err("block models did not resolve any sprite layers".into());
    }

    println!(
        "Validated Minecraft 1.21.1 runtime resources: {} states, {} blocks, {} built-in registries, {} block tags, {} sprite layers",
        registry.len(),
        blocks.len(),
        builtin.registries.len(),
        tags.len(),
        sprites.layers.len()
    );
    Ok(())
}
