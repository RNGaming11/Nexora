use std::{env, fs, path::PathBuf, process};

use horizon_client_render::assets::Assets;
use horizon_protocol::registry::Builtin;

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

    let assets = Assets::load(&root)?;
    let gameplay: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("gameplay-facts.json"))?)?;
    let gameplay_states = gameplay["states"]
        .as_array()
        .ok_or("gameplay facts have no state table")?;
    if gameplay_states.len() != assets.registry.len()
        || manifest["gameplay_fact_state_count"].as_u64() != Some(gameplay_states.len() as u64)
    {
        return Err("gameplay facts do not match the 1.21.1 block-state registry".into());
    }
    let behaviours: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("block-behaviours.json"))?)?;
    if behaviours["blocks"].as_array().map(Vec::len)
        != manifest["block_behaviour_block_count"]
            .as_u64()
            .map(|n| n as usize)
        || behaviours["states"].as_array().map(Vec::len)
            != manifest["block_behaviour_state_count"]
                .as_u64()
                .map(|n| n as usize)
        || behaviours["states"].as_array().map(Vec::len) != Some(assets.registry.len())
    {
        return Err("block behaviour facts do not match the 1.21.1 block registry".into());
    }

    let entity_types: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("entity-types.json"))?)?;
    if entity_types.as_object().map(serde_json::Map::len)
        != manifest["entity_type_count"].as_u64().map(|n| n as usize)
    {
        return Err("entity type facts do not match the 1.21.1 entity registry".into());
    }

    let render = assets.reload(&root)?;
    if render.sprite_layers.is_empty() {
        return Err("block models did not resolve any sprite layers".into());
    }

    println!(
        "Validated Minecraft 1.21.1 through the game resource loader: {} block states, {} built-in registries, {} sprite layers",
        assets.registry.len(),
        builtin.registries.len(),
        render.sprite_layers.len()
    );
    Ok(())
}
