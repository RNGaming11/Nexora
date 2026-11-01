//! Minecraft Java Edition 1.21.1 (protocol 767) packet codecs that must
//! not be aliased to the 26.3 packet definitions in this crate.
//!
//! This module is deliberately version-scoped. It does not change the active
//! protocol IDs or the shared client connection. Recipe definitions use an
//! explicit serializer registry because their payloads are registry-driven.
use crate::{
    Error, Result,
    buf::{Codec, Reader, Writer, pack_block_pos, unpack_block_pos},
    chunk::{
        ChunkData as ActiveChunkData, LightData as ActiveLightData, Paletted, Section, Strategy,
    },
    packets::{
        Empty,
        configuration as cfg,
        login as login_packets,
        login::GameProfile,
        play as play_packets,
        recipes::RecipeBookSettings,
    },
};
use horizon_world::nbt::Tag;

#[path = "../../../../ports/1.21.1/data_component_ids.rs"]
pub mod component_ids;
#[path = "v1_21_1/inventory.rs"]
pub mod inventory;
#[path = "v1_21_1/recipes.rs"]
mod recipe_codecs;
pub use recipe_codecs::VanillaRecipeSerializers;

pub const PROTOCOL_VERSION: i32 = 767;

fn read_string_list(r: &mut Reader, max_count: usize, max_chars: usize) -> Result<Vec<String>> {
    let n = r.count(max_count)?;
    (0..n).map(|_| r.string(max_chars)).collect()
}
fn write_string_list(values: &[String], w: &mut Writer) {
    w.varint(values.len() as i32);
    for value in values { w.string(value); }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomReportDetails { pub details: Vec<(String, String)> }
impl Codec<CustomReportDetails> for CustomReportDetails {
    fn read(r: &mut Reader) -> Result<Self> {
        let n = r.count(32)?;
        let mut details = Vec::with_capacity(n);
        for _ in 0..n { details.push((r.string(128)?, r.string(4096)?)); }
        Ok(Self { details })
    }
    fn write(v: &Self, w: &mut Writer) {
        w.varint(v.details.len() as i32);
        for (key, value) in &v.details { w.string(key); w.string(value); }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameProfilePacket { pub profile: GameProfile, pub strict_error_handling: bool }
impl Codec<GameProfilePacket> for GameProfilePacket {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self { profile: GameProfile::read(r)?, strict_error_handling: r.bool()? })
    }
    fn write(v: &Self, w: &mut Writer) {
        GameProfile::write(&v.profile, w);
        w.bool(v.strict_error_handling);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlobalPos1211 {
    pub dimension: String,
    pub position: [i32; 3],
}

impl Codec<GlobalPos1211> for GlobalPos1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            dimension: r.string(32767)?,
            position: unpack_block_pos(r.i64()?),
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.string(&value.dimension);
        w.i64(pack_block_pos(value.position));
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpawnInfo1211 {
    pub dimension_type: i32,
    pub dimension: String,
    pub seed: i64,
    pub game_type: i32,
    pub previous_game_type: Option<i32>,
    pub debug: bool,
    pub flat: bool,
    pub last_death: Option<GlobalPos1211>,
    pub portal_cooldown: i32,
}

impl Codec<SpawnInfo1211> for SpawnInfo1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        let dimension_type = r.varint()?;
        let dimension = r.string(32767)?;
        let seed = r.i64()?;
        let game_type = i32::from(r.u8()?);
        let previous_game_type = {
            let value = r.i8()?;
            (value != -1).then_some(i32::from(value))
        };
        let debug = r.bool()?;
        let flat = r.bool()?;
        let last_death = if r.bool()? {
            Some(GlobalPos1211::read(r)?)
        } else {
            None
        };
        let portal_cooldown = r.varint()?;
        Ok(Self {
            dimension_type,
            dimension,
            seed,
            game_type,
            previous_game_type,
            debug,
            flat,
            last_death,
            portal_cooldown,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.varint(value.dimension_type);
        w.string(&value.dimension);
        w.i64(value.seed);
        w.u8(value.game_type as u8);
        w.i8(value.previous_game_type.map_or(-1, |v| v as i8));
        w.bool(value.debug);
        w.bool(value.flat);
        w.bool(value.last_death.is_some());
        if let Some(position) = &value.last_death {
            GlobalPos1211::write(position, w);
        }
        w.varint(value.portal_cooldown);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Login1211 {
    pub player_id: i32,
    pub hardcore: bool,
    pub levels: Vec<String>,
    pub max_players: i32,
    pub chunk_radius: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub show_death_screen: bool,
    pub limited_crafting: bool,
    pub spawn: SpawnInfo1211,
    pub enforces_secure_chat: bool,
}

impl Codec<Login1211> for Login1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            player_id: r.i32()?,
            hardcore: r.bool()?,
            levels: read_string_list(r, 4096, 32767)?,
            max_players: r.varint()?,
            chunk_radius: r.varint()?,
            simulation_distance: r.varint()?,
            reduced_debug_info: r.bool()?,
            show_death_screen: r.bool()?,
            limited_crafting: r.bool()?,
            spawn: SpawnInfo1211::read(r)?,
            enforces_secure_chat: r.bool()?,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.i32(value.player_id);
        w.bool(value.hardcore);
        write_string_list(&value.levels, w);
        w.varint(value.max_players);
        w.varint(value.chunk_radius);
        w.varint(value.simulation_distance);
        w.bool(value.reduced_debug_info);
        w.bool(value.show_death_screen);
        w.bool(value.limited_crafting);
        SpawnInfo1211::write(&value.spawn, w);
        w.bool(value.enforces_secure_chat);
    }
}

pub fn decode_play_login(payload: &[u8]) -> Result<Login1211> {
    decode_exact(payload, Login1211::read)
}

crate::packet!(AddExperienceOrb {
    entity_id: i32 => crate::buf::VarInt,
    x: f64 => crate::buf::Double,
    y: f64 => crate::buf::Double,
    z: f64 => crate::buf::Double,
    value: i16 => crate::buf::Short,
});

#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion { pub text: String, pub tooltip: Option<Tag> }
impl Codec<Suggestion> for Suggestion {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self { text: r.string(32767)?, tooltip: if r.bool()? { Some(r.nbt()?) } else { None } })
    }
    fn write(v: &Self, w: &mut Writer) {
        w.string(&v.text);
        w.bool(v.tooltip.is_some());
        if let Some(tag) = &v.tooltip { w.nbt(tag); }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CommandSuggestions {
    pub transaction_id: i32, pub start: i32, pub length: i32, pub matches: Vec<Suggestion>,
}
impl Codec<CommandSuggestions> for CommandSuggestions {
    fn read(r: &mut Reader) -> Result<Self> {
        let transaction_id = r.varint()?;
        let start = r.varint()?;
        let length = r.varint()?;
        let n = r.count(32767)?;
        let mut matches = Vec::with_capacity(n);
        for _ in 0..n { matches.push(Suggestion::read(r)?); }
        Ok(Self { transaction_id, start, length, matches })
    }
    fn write(v: &Self, w: &mut Writer) {
        w.varint(v.transaction_id); w.varint(v.start); w.varint(v.length);
        w.varint(v.matches.len() as i32);
        for item in &v.matches { Suggestion::write(item, w); }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DebugSample { pub samples: Vec<i64>, pub sample_type: i32 }
impl Codec<DebugSample> for DebugSample {
    fn read(r: &mut Reader) -> Result<Self> {
        let samples = r.long_array()?;
        let sample_type = r.varint()?;
        Ok(Self { samples, sample_type })
    }
    fn write(v: &Self, w: &mut Writer) { w.long_array(&v.samples); w.varint(v.sample_type); }
}

crate::packet!(HorseScreenOpen {
    container_id: i32 => crate::buf::VarInt,
    size: i32 => crate::buf::VarInt,
    entity_id: i32 => crate::buf::Int,
});
crate::packet!(PlayerCombatEnd { duration: i32 => crate::buf::VarInt });

#[derive(Clone, Debug, PartialEq)]
pub struct SelectAdvancementsTab { pub tab: Option<String> }
impl Codec<SelectAdvancementsTab> for SelectAdvancementsTab {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self { tab: if r.bool()? { Some(r.string(32767)?) } else { None } })
    }
    fn write(v: &Self, w: &mut Writer) {
        w.bool(v.tab.is_some());
        if let Some(tab) = &v.tab { w.string(tab); }
    }
}

crate::packet!(SetCarriedItem { slot: i16 => crate::buf::Short });

crate::packet!(CommandSuggestionRequest {
    transaction_id: i32 => crate::buf::VarInt,
    text: String => crate::buf::StrN<32500>,
});
crate::packet!(DebugSampleSubscription { sample_type: i32 => crate::buf::VarInt });
crate::packet!(PickItem { slot: i32 => crate::buf::VarInt });
crate::packet!(RecipeBookSeenRecipe { recipe: String => crate::buf::Ident });

#[derive(Clone, Debug, PartialEq)]
pub struct SeenAdvancements { pub action: i32, pub tab: Option<String> }
impl Codec<SeenAdvancements> for SeenAdvancements {
    fn read(r: &mut Reader) -> Result<Self> {
        let action = r.varint()?;
        let tab = if action == 0 { Some(r.string(32767)?) } else { None };
        if !(0..=1).contains(&action) { return Err(Error::new("invalid seen-advancements action")); }
        Ok(Self { action, tab })
    }
    fn write(v: &Self, w: &mut Writer) {
        w.varint(v.action);
        if v.action == 0 { w.string(v.tab.as_deref().unwrap_or("")); }
    }
}

crate::packet!(Swing { hand: i32 => crate::buf::VarInt });

#[derive(Clone, Debug, PartialEq)]
pub struct RecipePacket {
    pub action: i32,
    pub recipes: Vec<String>,
    pub highlights: Vec<String>,
    pub book_settings: RecipeBookSettings,
}
impl Codec<RecipePacket> for RecipePacket {
    fn read(r: &mut Reader) -> Result<Self> {
        let action = r.varint()?;
        if !(0..=2).contains(&action) { return Err(Error::new("invalid recipe packet action")); }
        let recipes = read_string_list(r, 1_000_000, 32767)?;
        let highlights = read_string_list(r, 1_000_000, 32767)?;
        let book_settings = RecipeBookSettings::read(r)?;
        Ok(Self { action, recipes, highlights, book_settings })
    }
    fn write(v: &Self, w: &mut Writer) {
        w.varint(v.action);
        write_string_list(&v.recipes, w);
        write_string_list(&v.highlights, w);
        RecipeBookSettings::write(&v.book_settings, w);
    }
}

/// A recipe definition is an identifier, serializer identifier, and a
/// serializer-specific body. 1.21.1 uses RegistryFriendlyByteBuf here, so
/// guessing a generic body layout would silently desynchronize every packet
/// after the first recipe. Callers must register the exact serializer codec.
pub trait RecipeSerializer {
    fn read_recipe(&self, serializer: &str, id: &str, input: &mut Reader) -> Result<Vec<u8>>;
    fn write_recipe(&self, serializer: &str, id: &str, body: &[u8], output: &mut Writer) -> Result<()>;
}
#[derive(Clone, Debug, PartialEq)]
pub struct RecipeDefinition { pub id: String, pub serializer: String, pub body: Vec<u8> }
#[derive(Clone, Debug, PartialEq)]
pub struct UpdateRecipes { pub recipes: Vec<RecipeDefinition> }
impl UpdateRecipes {
    pub fn read_with(r: &mut Reader, serializers: &impl RecipeSerializer) -> Result<Self> {
        let count = r.count(1_000_000)?;
        let mut recipes = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            let id = r.string(32767)?;
            let serializer_id = r.varint()?;
            let serializer = recipe_codecs::serializer_name(serializer_id)?.to_owned();
            let body = serializers.read_recipe(&serializer, &id, r)?;
            recipes.push(RecipeDefinition { id, serializer, body });
        }
        Ok(Self { recipes })
    }
    pub fn write_with(&self, w: &mut Writer, serializers: &impl RecipeSerializer) -> Result<()> {
        w.varint(self.recipes.len() as i32);
        for recipe in &self.recipes {
            w.string(&recipe.id);
            w.varint(recipe_codecs::serializer_id(&recipe.serializer)?);
            serializers.write_recipe(&recipe.serializer, &recipe.id, &recipe.body, w)?;
        }
        Ok(())
    }
}

/// IDs from the generated 1.21.1 packet inventory. They intentionally live
/// here instead of crates/horizon-protocol/src/ids.rs (protocol 777).
pub mod ids {
    pub mod configuration_clientbound {
        pub const COOKIE_REQUEST: i32 = 0;
        pub const CUSTOM_PAYLOAD: i32 = 1;
        pub const DISCONNECT: i32 = 2;
        pub const FINISH_CONFIGURATION: i32 = 3;
        pub const KEEP_ALIVE: i32 = 4;
        pub const PING: i32 = 5;
        pub const RESET_CHAT: i32 = 6;
        pub const REGISTRY_DATA: i32 = 7;
        pub const RESOURCE_PACK_POP: i32 = 8;
        pub const RESOURCE_PACK_PUSH: i32 = 9;
        pub const STORE_COOKIE: i32 = 10;
        pub const TRANSFER: i32 = 11;
        pub const UPDATE_ENABLED_FEATURES: i32 = 12;
        pub const UPDATE_TAGS: i32 = 13;
        pub const SELECT_KNOWN_PACKS: i32 = 14;
        pub const CUSTOM_REPORT_DETAILS: i32 = 15;
        pub const SERVER_LINKS: i32 = 16;
        pub const NAMES: &[&str] = &[
            "cookie_request", "custom_payload", "disconnect", "finish_configuration",
            "keep_alive", "ping", "reset_chat", "registry_data", "resource_pack_pop",
            "resource_pack_push", "store_cookie", "transfer", "update_enabled_features",
            "update_tags", "select_known_packs", "custom_report_details", "server_links",
        ];
    }
    pub mod configuration_serverbound {
        pub const CLIENT_INFORMATION: i32 = 0;
        pub const COOKIE_RESPONSE: i32 = 1;
        pub const CUSTOM_PAYLOAD: i32 = 2;
        pub const FINISH_CONFIGURATION: i32 = 3;
        pub const KEEP_ALIVE: i32 = 4;
        pub const PONG: i32 = 5;
        pub const RESOURCE_PACK: i32 = 6;
        pub const SELECT_KNOWN_PACKS: i32 = 7;
        pub const NAMES: &[&str] = &[
            "client_information", "cookie_response", "custom_payload", "finish_configuration",
            "keep_alive", "pong", "resource_pack", "select_known_packs",
        ];
    }
    pub mod login_clientbound { pub const GAME_PROFILE: i32 = 2; }
    pub mod play_clientbound {
        pub const BUNDLE_DELIMITER: i32 = 0;
        pub const COOKIE_REQUEST: i32 = 22;
        pub const CUSTOM_PAYLOAD: i32 = 25;
        pub const DISCONNECT: i32 = 29;
        pub const KEEP_ALIVE: i32 = 38;
        pub const LOGIN: i32 = 43;
        pub const RESOURCE_PACK_POP: i32 = 69;
        pub const RESOURCE_PACK_PUSH: i32 = 70;
        pub const START_CONFIGURATION: i32 = 105;
        pub const STORE_COOKIE: i32 = 107;
        pub const TRANSFER: i32 = 115;
        pub const SERVER_LINKS: i32 = 123;
        pub const ADD_EXPERIENCE_ORB: i32 = 2;
        pub const COMMAND_SUGGESTIONS: i32 = 16;
        pub const CUSTOM_REPORT_DETAILS: i32 = 122;
        pub const DEBUG_SAMPLE: i32 = 27;
        pub const HORSE_SCREEN_OPEN: i32 = 35;
        pub const PLAYER_COMBAT_END: i32 = 58;
        pub const PLAYER_COMBAT_ENTER: i32 = 59;
        pub const RECIPE: i32 = 65;
        pub const SELECT_ADVANCEMENTS_TAB: i32 = 74;
        pub const SET_CARRIED_ITEM: i32 = 83;
        pub const UPDATE_RECIPES: i32 = 119;
    }
    pub mod play_serverbound {
        pub const COMMAND_SUGGESTION: i32 = 11;
        pub const DEBUG_SAMPLE_SUBSCRIPTION: i32 = 19;
        pub const PICK_ITEM: i32 = 32;
        pub const RECIPE_BOOK_SEEN_RECIPE: i32 = 41;
        pub const SEEN_ADVANCEMENTS: i32 = 44;
        pub const SWING: i32 = 54;
    }
}

/// Decode the straightforward 1.21.1 missing packets by their target IDs.
/// Recipe definitions are returned as a distinct "registry codec required"
/// error until a caller supplies the matching serializer registry.
#[derive(Clone, Debug, PartialEq)]
pub enum MissingPacket {
    Login(Login1211),
    AddExperienceOrb(AddExperienceOrb),
    CommandSuggestions(CommandSuggestions),
    CustomReportDetails(CustomReportDetails),
    DebugSample(DebugSample),
    HorseScreenOpen(HorseScreenOpen),
    PlayerCombatEnd(PlayerCombatEnd),
    PlayerCombatEnter(Empty),
    Recipe(RecipePacket),
    SelectAdvancementsTab(SelectAdvancementsTab),
    SetCarriedItem(SetCarriedItem),
    CommandSuggestionRequest(CommandSuggestionRequest),
    DebugSampleSubscription(DebugSampleSubscription),
    PickItem(PickItem),
    RecipeBookSeenRecipe(RecipeBookSeenRecipe),
    SeenAdvancements(SeenAdvancements),
    Swing(Swing),
}


#[derive(Clone, Debug, PartialEq)]
pub enum PlayPacket1211 {
    BundleDelimiter(Empty),
    CookieRequest(cfg::CookieRequest),
    CustomPayload(cfg::CustomPayload),
    Disconnect(cfg::Disconnect),
    KeepAlive(cfg::KeepAlive),
    Login(Login1211),
    ResourcePackPop(cfg::ResourcePackPop),
    ResourcePackPush(cfg::ResourcePackPush),
    ServerLinks(cfg::ServerLinks),
    StartConfiguration(Empty),
    StoreCookie(cfg::StoreCookie),
    Transfer(cfg::Transfer),
    Missing(MissingPacket),
    Inventory(inventory::Clientbound),
    Compatible(play_packets::Clientbound),
    LevelChunkWithLight(play_packets::LevelChunkWithLight),
    LightUpdate(play_packets::LightUpdate),
    PlayerPosition(play_packets::PlayerPosition),
    ChangeDifficulty(play_packets::ChangeDifficulty),
    SpawnPosition(SpawnPosition1211),
    EntityMotion(play_packets::SetEntityMotion),
    TeleportEntity(play_packets::TeleportEntity),
    MoveEntityPos(play_packets::MoveEntityPos),
    MoveEntityPosRot(play_packets::MoveEntityPosRot),
    MoveEntityRot(play_packets::MoveEntityRot),
}

/// Decode the 1.21.1 play packets whose payloads have target-verified codecs.
/// This is not a complete play-state decoder; unsupported IDs stay explicit
/// errors rather than being passed to the 26.3 packet table.
pub fn decode_play_packet_1_21_1(id: i32, payload: &[u8]) -> Result<PlayPacket1211> {
    let shared = |decode: fn(&mut Reader) -> Result<PlayPacket1211>| {
        decode_exact(payload, decode)
    };
    match id {
        ids::play_clientbound::BUNDLE_DELIMITER =>
            shared(|r| Empty::read(r).map(PlayPacket1211::BundleDelimiter)),
        ids::play_clientbound::COOKIE_REQUEST =>
            shared(|r| cfg::CookieRequest::read(r).map(PlayPacket1211::CookieRequest)),
        ids::play_clientbound::CUSTOM_PAYLOAD =>
            shared(|r| cfg::CustomPayload::read(r).map(PlayPacket1211::CustomPayload)),
        ids::play_clientbound::DISCONNECT =>
            shared(|r| cfg::Disconnect::read(r).map(PlayPacket1211::Disconnect)),
        ids::play_clientbound::KEEP_ALIVE =>
            shared(|r| cfg::KeepAlive::read(r).map(PlayPacket1211::KeepAlive)),
        id if matches!(
            id,
            crate::packet_ids_1_21_1::play::clientbound::CONTAINER_SET_CONTENT
                | crate::packet_ids_1_21_1::play::clientbound::CONTAINER_SET_SLOT
                | crate::packet_ids_1_21_1::play::clientbound::SET_EQUIPMENT
        ) => inventory::decode_clientbound(id, payload).map(PlayPacket1211::Inventory),
        crate::packet_ids_1_21_1::play::clientbound::LEVEL_CHUNK_WITH_LIGHT => {
            decode_level_chunk_with_light_1_21_1(payload)
                .map(PlayPacket1211::LevelChunkWithLight)
        }
        crate::packet_ids_1_21_1::play::clientbound::LIGHT_UPDATE => {
            decode_light_update_1_21_1(payload).map(PlayPacket1211::LightUpdate)
        }
        crate::packet_ids_1_21_1::play::clientbound::PLAYER_POSITION => {
            decode_player_position_1_21_1(payload).map(PlayPacket1211::PlayerPosition)
        }
        crate::packet_ids_1_21_1::play::clientbound::CHANGE_DIFFICULTY => {
            decode_change_difficulty_1_21_1(payload).map(PlayPacket1211::ChangeDifficulty)
        }
        crate::packet_ids_1_21_1::play::clientbound::SET_DEFAULT_SPAWN_POSITION => {
            decode_spawn_position_1_21_1(payload).map(PlayPacket1211::SpawnPosition)
        }
        crate::packet_ids_1_21_1::play::clientbound::SET_ENTITY_MOTION => {
            decode_entity_motion_1_21_1(payload).map(PlayPacket1211::EntityMotion)
        }
        crate::packet_ids_1_21_1::play::clientbound::TELEPORT_ENTITY => {
            decode_teleport_entity_1_21_1(payload).map(PlayPacket1211::TeleportEntity)
        }
        crate::packet_ids_1_21_1::play::clientbound::MOVE_ENTITY_POS => {
            decode_move_entity_pos_1_21_1(payload).map(PlayPacket1211::MoveEntityPos)
        }
        crate::packet_ids_1_21_1::play::clientbound::MOVE_ENTITY_POS_ROT => {
            decode_move_entity_pos_rot_1_21_1(payload).map(PlayPacket1211::MoveEntityPosRot)
        }
        crate::packet_ids_1_21_1::play::clientbound::MOVE_ENTITY_ROT => {
            decode_move_entity_rot_1_21_1(payload).map(PlayPacket1211::MoveEntityRot)
        }
        ids::play_clientbound::LOGIN =>
            shared(|r| Login1211::read(r).map(PlayPacket1211::Login)),
        ids::play_clientbound::RESOURCE_PACK_POP =>
            shared(|r| cfg::ResourcePackPop::read(r).map(PlayPacket1211::ResourcePackPop)),
        ids::play_clientbound::RESOURCE_PACK_PUSH =>
            shared(|r| cfg::ResourcePackPush::read(r).map(PlayPacket1211::ResourcePackPush)),
        ids::play_clientbound::SERVER_LINKS =>
            shared(|r| cfg::ServerLinks::read(r).map(PlayPacket1211::ServerLinks)),
        ids::play_clientbound::START_CONFIGURATION =>
            shared(|r| Empty::read(r).map(PlayPacket1211::StartConfiguration)),
        ids::play_clientbound::STORE_COOKIE =>
            shared(|r| cfg::StoreCookie::read(r).map(PlayPacket1211::StoreCookie)),
        ids::play_clientbound::TRANSFER =>
            shared(|r| cfg::Transfer::read(r).map(PlayPacket1211::Transfer)),
        id if is_missing_play_clientbound(id) =>
            decode_play_clientbound(id, payload).map(PlayPacket1211::Missing),
        id => decode_compatible_play_packet_1_21_1(id, payload)
            .map(PlayPacket1211::Compatible),

    }
}

/// The 1.21.1 chunk packet stores heightmaps as anonymous NBT, uses a
/// single block-count short per section, 15-bit global block states, 6-bit
/// global biomes, and 64-bit light masks. This codec converts the section
/// storage to the active renderer's 26.3-compatible in-memory representation.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelChunkWithLight1211 {
    pub x: i32,
    pub z: i32,
    pub heightmaps: Tag,
    pub chunk_data: Vec<u8>,
    pub block_entities: Vec<crate::chunk::BlockEntityInfo>,
    pub light: ActiveLightData,
}

fn read_target_light(r: &mut Reader) -> Result<ActiveLightData> {
    let read_mask = |r: &mut Reader| -> Result<Vec<u8>> {
        let count = r.count(8)?;
        let mut bytes = Vec::with_capacity(count * 8);
        for _ in 0..count {
            bytes.extend_from_slice(&r.i64()?.to_le_bytes());
        }
        Ok(bytes)
    };
    let read_layers = |r: &mut Reader| -> Result<Vec<Vec<u8>>> {
        let count = r.count(1)?;
        (0..count).map(|_| r.byte_array(2048)).collect()
    };
    Ok(ActiveLightData {
        sky_mask: read_mask(r)?,
        block_mask: read_mask(r)?,
        empty_sky_mask: read_mask(r)?,
        empty_block_mask: read_mask(r)?,
        sky: read_layers(r)?,
        block: read_layers(r)?,
    })
}

impl Codec<LevelChunkWithLight1211> for LevelChunkWithLight1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        let x = r.i32()?;
        let z = r.i32()?;
        let heightmaps = r.nbt()?;
        if !matches!(heightmaps, Tag::Compound(_)) {
            return Err(Error::new("protocol-767 chunk heightmaps were not a compound"));
        }
        let chunk_data = r.byte_array(16 * 1024 * 1024)?;
        let entity_count = r.count(5)?;
        let block_entities = (0..entity_count)
            .map(|_| crate::chunk::BlockEntityInfo::read(r))
            .collect::<Result<Vec<_>>>()?;
        let light = read_target_light(r)?;
        Ok(Self {
            x,
            z,
            heightmaps,
            chunk_data,
            block_entities,
            light,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.i32(value.x);
        w.i32(value.z);
        w.nbt(&value.heightmaps);
        w.byte_array(&value.chunk_data);
        w.varint(value.block_entities.len() as i32);
        for entity in &value.block_entities {
            crate::chunk::BlockEntityInfo::write(entity, w);
        }
        write_target_light(&value.light, w);
    }
}

fn write_target_light(light: &ActiveLightData, w: &mut Writer) {
    let write_mask = |mask: &[u8], w: &mut Writer| {
        let mut bytes = mask.to_vec();
        bytes.resize(bytes.len().div_ceil(8) * 8, 0);
        w.varint((bytes.len() / 8) as i32);
        for word in bytes.chunks_exact(8) {
            w.i64(i64::from_le_bytes(word.try_into().unwrap()));
        }
    };
    let write_layers = |layers: &[Vec<u8>], w: &mut Writer| {
        w.varint(layers.len() as i32);
        for layer in layers {
            w.byte_array(layer);
        }
    };
    write_mask(&light.sky_mask, w);
    write_mask(&light.block_mask, w);
    write_mask(&light.empty_sky_mask, w);
    write_mask(&light.empty_block_mask, w);
    write_layers(&light.sky, w);
    write_layers(&light.block, w);
}

fn promote_target_chunk_sections(data: &[u8]) -> Result<Vec<u8>> {
    let mut r = Reader::new(data);
    let mut sections = Vec::new();
    while r.remaining() > 0 {
        if sections.len() >= 1024 {
            return Err(Error::new("protocol-767 chunk contains too many sections"));
        }
        let non_empty = r.i16()?;
        let wire_blocks = Paletted::read(&mut r, Strategy::Blocks, 15)?;
        let wire_biomes = Paletted::read(&mut r, Strategy::Biomes, 6)?;
        let block_values = (0..crate::chunk::BLOCKS)
            .map(|index| wire_blocks.get(index))
            .collect::<Vec<_>>();
        let biome_values = (0..crate::chunk::BIOMES)
            .map(|index| wire_biomes.get(index))
            .collect::<Vec<_>>();
        sections.push(Section {
            non_empty,
            fluids: 0,
            states: Paletted::from_values(&block_values, Strategy::Blocks, 16),
            biomes: Paletted::from_values(&biome_values, Strategy::Biomes, 7),
        });
    }
    Ok(ActiveChunkData::from_sections(&sections))
}

pub fn decode_level_chunk_with_light_1_21_1(
    payload: &[u8],
) -> Result<play_packets::LevelChunkWithLight> {
    let mut r = Reader::new(payload);
    let value = LevelChunkWithLight1211::read(&mut r)?;
    if r.remaining() != 0 {
        return Err(Error::new(format!(
            "protocol-767 chunk packet has {} trailing bytes",
            r.remaining()
        )));
    }
    let buffer = promote_target_chunk_sections(&value.chunk_data)?;
    Ok(play_packets::LevelChunkWithLight {
        x: value.x,
        z: value.z,
        chunk: ActiveChunkData {
            heightmaps: Vec::new(),
            buffer,
            block_entities: value.block_entities,
        },
        light: value.light,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightUpdate1211 {
    pub x: i32,
    pub z: i32,
    pub light: ActiveLightData,
}

impl Codec<LightUpdate1211> for LightUpdate1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            x: r.varint()?,
            z: r.varint()?,
            light: read_target_light(r)?,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.varint(value.x);
        w.varint(value.z);
        write_target_light(&value.light, w);
    }
}

pub fn decode_light_update_1_21_1(payload: &[u8]) -> Result<play_packets::LightUpdate> {
    let mut r = Reader::new(payload);
    let value = LightUpdate1211::read(&mut r)?;
    if r.remaining() != 0 {
        return Err(Error::new(format!(
            "protocol-767 light packet has {} trailing bytes",
            r.remaining()
        )));
    }
    Ok(play_packets::LightUpdate {
        x: value.x,
        z: value.z,
        light: value.light,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerPosition1211 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub relatives: u8,
    pub teleport_id: i32,
}

impl Codec<PlayerPosition1211> for PlayerPosition1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            x: r.f64()?,
            y: r.f64()?,
            z: r.f64()?,
            yaw: r.f32()?,
            pitch: r.f32()?,
            relatives: r.u8()?,
            teleport_id: r.varint()?,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.f64(value.x);
        w.f64(value.y);
        w.f64(value.z);
        w.f32(value.yaw);
        w.f32(value.pitch);
        w.u8(value.relatives);
        w.varint(value.teleport_id);
    }
}

pub fn decode_player_position_1_21_1(payload: &[u8]) -> Result<play_packets::PlayerPosition> {
    let value = decode_exact(payload, PlayerPosition1211::read)?;
    Ok(play_packets::PlayerPosition {
        teleport: value.teleport_id,
        position: [value.x, value.y, value.z],
        delta: [0.0; 3],
        yaw: value.yaw,
        pitch: value.pitch,
        relatives: i32::from(value.relatives),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChangeDifficulty1211 {
    pub difficulty: u8,
    pub locked: bool,
}

impl Codec<ChangeDifficulty1211> for ChangeDifficulty1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            difficulty: r.u8()?,
            locked: r.bool()?,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.u8(value.difficulty);
        w.bool(value.locked);
    }
}

pub fn decode_change_difficulty_1_21_1(payload: &[u8]) -> Result<play_packets::ChangeDifficulty> {
    let value = decode_exact(payload, ChangeDifficulty1211::read)?;
    Ok(play_packets::ChangeDifficulty {
        difficulty: i32::from(value.difficulty),
        locked: value.locked,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpawnPosition1211 {
    pub position: [i32; 3],
    pub angle: f32,
}

impl Codec<SpawnPosition1211> for SpawnPosition1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            position: unpack_block_pos(r.i64()?),
            angle: r.f32()?,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.i64(pack_block_pos(value.position));
        w.f32(value.angle);
    }
}

pub fn decode_spawn_position_1_21_1(
    payload: &[u8],
) -> Result<SpawnPosition1211> {
    decode_exact(payload, SpawnPosition1211::read)
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityMotion1211 {
    pub entity_id: i32,
    pub velocity: [i16; 3],
}

impl Codec<EntityMotion1211> for EntityMotion1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            entity_id: r.varint()?,
            velocity: [r.i16()?, r.i16()?, r.i16()?],
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.varint(value.entity_id);
        for component in value.velocity {
            w.i16(component);
        }
    }
}

pub fn decode_entity_motion_1_21_1(
    payload: &[u8],
) -> Result<play_packets::SetEntityMotion> {
    let value = decode_exact(payload, EntityMotion1211::read)?;
    Ok(play_packets::SetEntityMotion {
        id: value.entity_id,
        velocity: value.velocity.map(|component| f64::from(component) / 8000.0),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct TeleportEntity1211 {
    pub entity_id: i32,
    pub position: [f64; 3],
    pub yaw: i8,
    pub pitch: i8,
    pub on_ground: bool,
}

impl Codec<TeleportEntity1211> for TeleportEntity1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            entity_id: r.varint()?,
            position: [r.f64()?, r.f64()?, r.f64()?],
            yaw: r.i8()?,
            pitch: r.i8()?,
            on_ground: r.bool()?,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.varint(value.entity_id);
        for coordinate in value.position {
            w.f64(coordinate);
        }
        w.i8(value.yaw);
        w.i8(value.pitch);
        w.bool(value.on_ground);
    }
}

pub fn decode_teleport_entity_1_21_1(
    payload: &[u8],
) -> Result<play_packets::TeleportEntity> {
    let value = decode_exact(payload, TeleportEntity1211::read)?;
    let angle = |byte: i8| f32::from(byte) * (360.0 / 256.0);
    Ok(play_packets::TeleportEntity {
        id: value.entity_id,
        position: value.position,
        delta: [0.0; 3],
        yaw: angle(value.yaw),
        pitch: angle(value.pitch),
        relatives: 0,
        on_ground: value.on_ground,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveEntityPos1211 {
    pub entity_id: i32,
    pub delta: [i16; 3],
    pub on_ground: bool,
}

impl Codec<MoveEntityPos1211> for MoveEntityPos1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            entity_id: r.varint()?,
            delta: [r.i16()?, r.i16()?, r.i16()?],
            on_ground: r.bool()?,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.varint(value.entity_id);
        for component in value.delta {
            w.i16(component);
        }
        w.bool(value.on_ground);
    }
}

pub fn decode_move_entity_pos_1_21_1(payload: &[u8]) -> Result<play_packets::MoveEntityPos> {
    let value = decode_exact(payload, MoveEntityPos1211::read)?;
    Ok(play_packets::MoveEntityPos(play_packets::MoveEntity {
        id: value.entity_id,
        delta: Some(play_packets::VecDelta::Linear(value.delta)),
        rotation: None,
        on_ground: value.on_ground,
    }))
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveEntityPosRot1211 {
    pub entity_id: i32,
    pub delta: [i16; 3],
    pub yaw: i8,
    pub pitch: i8,
    pub on_ground: bool,
}

impl Codec<MoveEntityPosRot1211> for MoveEntityPosRot1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            entity_id: r.varint()?,
            delta: [r.i16()?, r.i16()?, r.i16()?],
            yaw: r.i8()?,
            pitch: r.i8()?,
            on_ground: r.bool()?,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.varint(value.entity_id);
        for component in value.delta {
            w.i16(component);
        }
        w.i8(value.yaw);
        w.i8(value.pitch);
        w.bool(value.on_ground);
    }
}

pub fn decode_move_entity_pos_rot_1_21_1(
    payload: &[u8],
) -> Result<play_packets::MoveEntityPosRot> {
    let value = decode_exact(payload, MoveEntityPosRot1211::read)?;
    Ok(play_packets::MoveEntityPosRot(play_packets::MoveEntity {
        id: value.entity_id,
        delta: Some(play_packets::VecDelta::Linear(value.delta)),
        rotation: Some((value.yaw, value.pitch)),
        on_ground: value.on_ground,
    }))
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveEntityRot1211 {
    pub entity_id: i32,
    pub yaw: i8,
    pub pitch: i8,
    pub on_ground: bool,
}

impl Codec<MoveEntityRot1211> for MoveEntityRot1211 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            entity_id: r.varint()?,
            yaw: r.i8()?,
            pitch: r.i8()?,
            on_ground: r.bool()?,
        })
    }

    fn write(value: &Self, w: &mut Writer) {
        w.varint(value.entity_id);
        w.i8(value.yaw);
        w.i8(value.pitch);
        w.bool(value.on_ground);
    }
}

pub fn decode_move_entity_rot_1_21_1(payload: &[u8]) -> Result<play_packets::MoveEntityRot> {
    let value = decode_exact(payload, MoveEntityRot1211::read)?;
    Ok(play_packets::MoveEntityRot(play_packets::MoveEntity {
        id: value.entity_id,
        delta: None,
        rotation: Some((value.yaw, value.pitch)),
        on_ground: value.on_ground,
    }))
}

fn decode_compatible_play_packet_1_21_1(
    target_id: i32,
    payload: &[u8],
) -> Result<play_packets::Clientbound> {
    let name = crate::packet_ids_1_21_1::play::clientbound::NAMES
        .get(usize::try_from(target_id).map_err(|_| Error::new("negative protocol-767 packet id"))?)
        .copied()
        .ok_or_else(|| Error::new(format!("unknown protocol-767 clientbound play packet: {target_id}")))?;
    if !matches!(
        name,
        "animate"
            | "block_event"
            | "block_update"
            | "chunk_batch_finished"
            | "chunk_batch_start"
            | "container_close"
            | "game_event"
            | "hurt_animation"
            | "level_event"
            | "move_vehicle"
            | "pong_response"
            | "remove_mob_effect"
            | "reset_score"
            | "set_action_bar_text"
            | "set_border_center"
            | "set_border_lerp_size"
            | "set_border_size"
            | "set_border_warning_delay"
            | "set_border_warning_distance"
            | "set_camera"
            | "set_chunk_cache_center"
            | "set_chunk_cache_radius"
            | "set_display_objective"
            | "set_entity_link"
            | "set_experience"
            | "set_health"
            | "set_objective"
            | "set_passengers"
            | "set_simulation_distance"
            | "set_subtitle_text"
            | "set_title_text"
            | "set_titles_animation"
            | "system_chat"
            | "tab_list"
            | "take_item_entity"
            | "set_projectile_power"
            | "ping"
            | "player_abilities"
    ) {
        return Err(Error::new(format!(
            "protocol-767 packet {name} has no verified compatible payload adapter"
        )));
    }
    let active_id = crate::ids::play::clientbound::NAMES
        .iter()
        .position(|active_name| *active_name == name)
        .ok_or_else(|| Error::new(format!("protocol-767 packet {name} is absent from the active codec table")))? as i32;
    play_packets::Clientbound::decode(active_id, payload)
}

fn is_missing_play_clientbound(id: i32) -> bool {
    matches!(
        id,
        ids::play_clientbound::ADD_EXPERIENCE_ORB
            | ids::play_clientbound::COMMAND_SUGGESTIONS
            | ids::play_clientbound::CUSTOM_REPORT_DETAILS
            | ids::play_clientbound::DEBUG_SAMPLE
            | ids::play_clientbound::HORSE_SCREEN_OPEN
            | ids::play_clientbound::PLAYER_COMBAT_END
            | ids::play_clientbound::PLAYER_COMBAT_ENTER
            | ids::play_clientbound::RECIPE
            | ids::play_clientbound::SELECT_ADVANCEMENTS_TAB
            | ids::play_clientbound::SET_CARRIED_ITEM
            | ids::play_clientbound::UPDATE_RECIPES
    )
}

/// Decode one of the missing 1.21.1 clientbound play packets whose payload
/// codec is fully represented above. Unknown IDs remain the caller's concern.
pub fn decode_play_clientbound(id: i32, payload: &[u8]) -> Result<MissingPacket> {
    let mut r = Reader::new(payload);
    let packet = match id {
        ids::play_clientbound::LOGIN => MissingPacket::Login(Login1211::read(&mut r)?),
        ids::play_clientbound::ADD_EXPERIENCE_ORB => MissingPacket::AddExperienceOrb(AddExperienceOrb::read(&mut r)?),
        ids::play_clientbound::COMMAND_SUGGESTIONS => MissingPacket::CommandSuggestions(CommandSuggestions::read(&mut r)?),
        ids::play_clientbound::CUSTOM_REPORT_DETAILS => MissingPacket::CustomReportDetails(CustomReportDetails::read(&mut r)?),
        ids::play_clientbound::DEBUG_SAMPLE => MissingPacket::DebugSample(DebugSample::read(&mut r)?),
        ids::play_clientbound::HORSE_SCREEN_OPEN => MissingPacket::HorseScreenOpen(HorseScreenOpen::read(&mut r)?),
        ids::play_clientbound::PLAYER_COMBAT_END => MissingPacket::PlayerCombatEnd(PlayerCombatEnd::read(&mut r)?),
        ids::play_clientbound::PLAYER_COMBAT_ENTER => MissingPacket::PlayerCombatEnter(Empty::read(&mut r)?),
        ids::play_clientbound::RECIPE => MissingPacket::Recipe(RecipePacket::read(&mut r)?),
        ids::play_clientbound::SELECT_ADVANCEMENTS_TAB => MissingPacket::SelectAdvancementsTab(SelectAdvancementsTab::read(&mut r)?),
        ids::play_clientbound::SET_CARRIED_ITEM => MissingPacket::SetCarriedItem(SetCarriedItem::read(&mut r)?),
        ids::play_clientbound::UPDATE_RECIPES => {
            return Err(Error::new("1.21.1 update_recipes requires a registry-aware RecipeSerializer"));
        }
        other => return Err(Error::new(format!("not a supported missing 1.21.1 clientbound play packet: {other}"))),
    };
    if r.remaining() != 0 { return Err(Error::new(format!("1.21.1 packet {id} has {} trailing bytes", r.remaining()))); }
    Ok(packet)
}

pub fn decode_play_serverbound(id: i32, payload: &[u8]) -> Result<MissingPacket> {
    let mut r = Reader::new(payload);
    let packet = match id {
        ids::play_serverbound::COMMAND_SUGGESTION => MissingPacket::CommandSuggestionRequest(CommandSuggestionRequest::read(&mut r)?),
        ids::play_serverbound::DEBUG_SAMPLE_SUBSCRIPTION => MissingPacket::DebugSampleSubscription(DebugSampleSubscription::read(&mut r)?),
        ids::play_serverbound::PICK_ITEM => MissingPacket::PickItem(PickItem::read(&mut r)?),
        ids::play_serverbound::RECIPE_BOOK_SEEN_RECIPE => MissingPacket::RecipeBookSeenRecipe(RecipeBookSeenRecipe::read(&mut r)?),
        ids::play_serverbound::SEEN_ADVANCEMENTS => MissingPacket::SeenAdvancements(SeenAdvancements::read(&mut r)?),
        ids::play_serverbound::SWING => MissingPacket::Swing(Swing::read(&mut r)?),
        other => return Err(Error::new(format!("not a supported missing 1.21.1 serverbound play packet: {other}"))),
    };
    if r.remaining() != 0 { return Err(Error::new(format!("1.21.1 packet {id} has {} trailing bytes", r.remaining()))); }
    Ok(packet)
}

/// Re-encode a shared play packet with its protocol-767 packet ID. Only
/// serverbound payloads whose layouts are verified for 1.21.1 are accepted.
pub fn encode_play_serverbound_1_21_1(packet: play_packets::Serverbound) -> Result<Vec<u8>> {
    match packet {
        play_packets::Serverbound::ContainerClick(value) => {
            let click = inventory::ContainerClick {
                container: value.container,
                state: value.state,
                slot: value.slot,
                button: value.button,
                mode: value.input,
                changed: value
                    .changed
                    .into_iter()
                    .map(|(slot, item)| inventory::ChangedSlot { slot, item })
                    .collect(),
                carried: value.carried,
            };
            return inventory::encode_serverbound(&inventory::Serverbound::Click(click));
        }
        play_packets::Serverbound::SetCreativeModeSlot(value) => {
            let slot = inventory::SetCreativeModeSlot {
                slot: value.slot,
                item: value.item,
            };
            return inventory::encode_serverbound(&inventory::Serverbound::CreativeSlot(slot));
        }
        packet => {
            let encoded = packet.encode();
    let mut reader = Reader::new(&encoded);
    let active_id = reader.varint()?;
    let name = crate::ids::play::serverbound::NAMES
        .get(usize::try_from(active_id).map_err(|_| Error::new("negative active play packet id"))?)
        .copied()
        .ok_or_else(|| Error::new(format!("unknown active play packet id {active_id}")))?;
    if !matches!(
        name,
        "accept_teleportation"
            | "chunk_batch_received"
            | "client_command"
            | "configuration_acknowledged"
            | "cookie_response"
            | "custom_payload"
            | "keep_alive"
            | "move_player_pos"
            | "move_player_pos_rot"
            | "move_player_rot"
            | "move_player_status_only"
            | "move_vehicle"
            | "player_abilities"
            | "player_action"
            | "player_command"
            | "player_input"
            | "player_loaded"
            | "pong"
            | "resource_pack"
            | "set_carried_item"
            | "use_item"
            | "use_item_on"
    ) {
        return Err(Error::new(format!(
            "protocol-767 serverbound packet {name} has no verified payload adapter"
        )));
    }
    let target_name = name;
    let target_id = crate::packet_ids_1_21_1::play::serverbound::NAMES
        .iter()
        .position(|candidate| *candidate == target_name)
        .ok_or_else(|| {
            Error::new(format!(
                "protocol-767 has no serverbound packet adapter for {name}"
            ))
        })?;

    let mut body = Writer::new();
    match name {
        "accept_teleportation" => {
            body.varint(reader.varint()?);
        }
        "move_player_pos" => {
            body.f64(reader.f64()?);
            body.f64(reader.f64()?);
            body.f64(reader.f64()?);
            let flags = reader.u8()?;
            body.bool(flags & 1 != 0);
        }
        "move_player_pos_rot" => {
            body.f64(reader.f64()?);
            body.f64(reader.f64()?);
            body.f64(reader.f64()?);
            body.f32(reader.f32()?);
            body.f32(reader.f32()?);
            let flags = reader.u8()?;
            body.bool(flags & 1 != 0);
        }
        "move_player_rot" => {
            body.f32(reader.f32()?);
            body.f32(reader.f32()?);
            let flags = reader.u8()?;
            body.bool(flags & 1 != 0);
        }
        "move_player_status_only" => {
            let flags = reader.u8()?;
            body.bool(flags & 1 != 0);
        }
        "move_vehicle" => {
            body.f64(reader.f64()?);
            body.f64(reader.f64()?);
            body.f64(reader.f64()?);
            body.f32(reader.f32()?);
            body.f32(reader.f32()?);
            let _on_ground = reader.bool()?;
        }
        "use_item_on" => {
            body.varint(reader.varint()?);
            body.i64(reader.i64()?);
            body.varint(reader.varint()?);
            body.f32(reader.f32()?);
            body.f32(reader.f32()?);
            body.f32(reader.f32()?);
            body.bool(reader.bool()?);
            let _world_border = reader.bool()?;
            body.varint(reader.varint()?);
        }
        "player_action" => {
            body.varint(reader.varint()?);
            body.i64(reader.i64()?);
            body.u8(reader.u8()?);
            body.varint(reader.varint()?);
        }
        _ => body.bytes(reader.rest()),
    }
    if reader.remaining() != 0 {
        return Err(Error::new(format!(
            "protocol-767 serverbound adapter for {name} left {} payload bytes",
            reader.remaining()
        )));
    }

    let mut out = Writer::new();
    out.varint(target_id as i32);
    out.bytes(&body.data);
    Ok(out.data)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoginPacket1211 {
    Shared(login_packets::Clientbound),
    GameProfile(GameProfilePacket),
}

pub fn decode_login_packet_1_21_1(id: i32, payload: &[u8]) -> Result<LoginPacket1211> {
    if id == ids::login_clientbound::GAME_PROFILE {
        return decode_exact(payload, GameProfilePacket::read)
            .map(LoginPacket1211::GameProfile);
    }
    login_packets::Clientbound::decode(id, payload).map(LoginPacket1211::Shared)
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigurationPacket1211 {
    Shared(cfg::Clientbound),
    CustomReportDetails(CustomReportDetails),
}

pub fn decode_configuration_packet_1_21_1(
    id: i32,
    payload: &[u8],
) -> Result<ConfigurationPacket1211> {
    if id == ids::configuration_clientbound::CUSTOM_REPORT_DETAILS {
        return decode_exact(payload, CustomReportDetails::read)
            .map(ConfigurationPacket1211::CustomReportDetails);
    }

    // Packet ids 0 through 9 retain their positions. The 26.3 protocol adds
    // post_effects at id 10, shifting the remaining shared packet codecs by one.
    let active_id = if id >= ids::configuration_clientbound::STORE_COOKIE {
        id + 1
    } else {
        id
    };
    cfg::Clientbound::decode(active_id, payload).map(ConfigurationPacket1211::Shared)
}

pub fn encode_configuration_serverbound_1_21_1(
    packet: cfg::Serverbound,
) -> Result<Vec<u8>> {
    match packet {
        cfg::Serverbound::ClientInformation(info) => {
            let mut writer = Writer::new();
            writer.varint(ids::configuration_serverbound::CLIENT_INFORMATION);
            writer.string(&info.language);
            writer.i8(info.view_distance);
            writer.varint(info.chat_visibility);
            writer.bool(info.chat_colors);
            writer.u8(info.model_customisation);
            writer.varint(info.main_hand);
            writer.bool(info.text_filtering);
            writer.bool(info.allows_listing);
            Ok(writer.data)
        }
        cfg::Serverbound::CookieResponse(_)
        | cfg::Serverbound::CustomPayload(_)
        | cfg::Serverbound::FinishConfiguration(_)
        | cfg::Serverbound::KeepAlive(_)
        | cfg::Serverbound::Pong(_)
        | cfg::Serverbound::ResourcePack(_)
        | cfg::Serverbound::SelectKnownPacks(_) => {
            let id = packet.id();
            if !(0..=7).contains(&id) {
                return Err(Error::new(format!(
                    "unsupported protocol-767 configuration packet id {id}"
                )));
            }
            Ok(packet.encode())
        }
        other => Err(Error::new(format!(
            "protocol-767 configuration packet has no adapter: {}",
            other.id()
        ))),
    }
}

pub fn decode_configuration_clientbound(id: i32, payload: &[u8]) -> Result<CustomReportDetails> {
    if id != ids::configuration_clientbound::CUSTOM_REPORT_DETAILS {
        return Err(Error::new(format!("not 1.21.1 configuration custom_report_details: {id}")));
    }
    decode_exact(payload, CustomReportDetails::read)
}

pub fn decode_login_clientbound(id: i32, payload: &[u8]) -> Result<GameProfilePacket> {
    if id != ids::login_clientbound::GAME_PROFILE {
        return Err(Error::new(format!("not 1.21.1 login game_profile: {id}")));
    }
    decode_exact(payload, GameProfilePacket::read)
}


fn decode_exact<T>(payload: &[u8], decode: impl FnOnce(&mut Reader) -> Result<T>) -> Result<T> {
    let mut r = Reader::new(payload);
    let value = decode(&mut r)?;
    if r.remaining() != 0 { return Err(Error::new(format!("packet has {} trailing bytes", r.remaining()))); }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<T: Codec<T> + PartialEq + std::fmt::Debug>(value: T) {
        let mut w = Writer::new();
        T::write(&value, &mut w);
        let mut r = Reader::new(&w.data);
        let decoded = T::read(&mut r).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn protocol_module_matches_the_isolated_target_version_facts() {
        assert_eq!(PROTOCOL_VERSION, horizon_core::version_1_21_1::PROTOCOL_VERSION);
        assert_eq!(horizon_core::version_1_21_1::VANILLA_VERSION, "1.21.1");
        assert_ne!(PROTOCOL_VERSION, crate::PROTOCOL_VERSION);
    }

    #[test]
    fn generated_packet_inventory_matches_protocol_767_codec_ids() {
        use crate::{Direction, State, packet_ids_1_21_1 as generated};

        assert_eq!(generated::handshake::serverbound::INTENTION, 0);
        assert_eq!(generated::name(State::Login, Direction::Clientbound, ids::login_clientbound::GAME_PROFILE), Some("game_profile"));

        for (id, name) in [
            (ids::configuration_clientbound::COOKIE_REQUEST, "cookie_request"),
            (ids::configuration_clientbound::CUSTOM_PAYLOAD, "custom_payload"),
            (ids::configuration_clientbound::DISCONNECT, "disconnect"),
            (ids::configuration_clientbound::FINISH_CONFIGURATION, "finish_configuration"),
            (ids::configuration_clientbound::KEEP_ALIVE, "keep_alive"),
            (ids::configuration_clientbound::PING, "ping"),
            (ids::configuration_clientbound::RESET_CHAT, "reset_chat"),
            (ids::configuration_clientbound::REGISTRY_DATA, "registry_data"),
            (ids::configuration_clientbound::RESOURCE_PACK_POP, "resource_pack_pop"),
            (ids::configuration_clientbound::RESOURCE_PACK_PUSH, "resource_pack_push"),
            (ids::configuration_clientbound::STORE_COOKIE, "store_cookie"),
            (ids::configuration_clientbound::TRANSFER, "transfer"),
            (ids::configuration_clientbound::UPDATE_ENABLED_FEATURES, "update_enabled_features"),
            (ids::configuration_clientbound::UPDATE_TAGS, "update_tags"),
            (ids::configuration_clientbound::SELECT_KNOWN_PACKS, "select_known_packs"),
            (ids::configuration_clientbound::CUSTOM_REPORT_DETAILS, "custom_report_details"),
            (ids::configuration_clientbound::SERVER_LINKS, "server_links"),
        ] {
            assert_eq!(generated::name(State::Configuration, Direction::Clientbound, id), Some(name));
        }

        for (id, name) in [
            (ids::configuration_serverbound::CLIENT_INFORMATION, "client_information"),
            (ids::configuration_serverbound::COOKIE_RESPONSE, "cookie_response"),
            (ids::configuration_serverbound::CUSTOM_PAYLOAD, "custom_payload"),
            (ids::configuration_serverbound::FINISH_CONFIGURATION, "finish_configuration"),
            (ids::configuration_serverbound::KEEP_ALIVE, "keep_alive"),
            (ids::configuration_serverbound::PONG, "pong"),
            (ids::configuration_serverbound::RESOURCE_PACK, "resource_pack"),
            (ids::configuration_serverbound::SELECT_KNOWN_PACKS, "select_known_packs"),
        ] {
            assert_eq!(generated::name(State::Configuration, Direction::Serverbound, id), Some(name));
        }

        for (id, name) in [
            (ids::play_clientbound::ADD_EXPERIENCE_ORB, "add_experience_orb"),
            (ids::play_clientbound::COMMAND_SUGGESTIONS, "command_suggestions"),
            (ids::play_clientbound::CUSTOM_REPORT_DETAILS, "custom_report_details"),
            (ids::play_clientbound::DEBUG_SAMPLE, "debug_sample"),
            (ids::play_clientbound::HORSE_SCREEN_OPEN, "horse_screen_open"),
            (ids::play_clientbound::PLAYER_COMBAT_END, "player_combat_end"),
            (ids::play_clientbound::PLAYER_COMBAT_ENTER, "player_combat_enter"),
            (ids::play_clientbound::RECIPE, "recipe"),
            (ids::play_clientbound::SELECT_ADVANCEMENTS_TAB, "select_advancements_tab"),
            (ids::play_clientbound::SET_CARRIED_ITEM, "set_carried_item"),
            (ids::play_clientbound::UPDATE_RECIPES, "update_recipes"),
        ] {
            assert_eq!(generated::name(State::Play, Direction::Clientbound, id), Some(name));
        }

        for (id, name) in [
            (ids::play_serverbound::COMMAND_SUGGESTION, "command_suggestion"),
            (ids::play_serverbound::DEBUG_SAMPLE_SUBSCRIPTION, "debug_sample_subscription"),
            (ids::play_serverbound::PICK_ITEM, "pick_item"),
            (ids::play_serverbound::RECIPE_BOOK_SEEN_RECIPE, "recipe_book_seen_recipe"),
            (ids::play_serverbound::SEEN_ADVANCEMENTS, "seen_advancements"),
            (ids::play_serverbound::SWING, "swing"),
        ] {
            assert_eq!(generated::name(State::Play, Direction::Serverbound, id), Some(name));
        }

        assert_eq!(generated::play::clientbound::LOGIN, ids::play_clientbound::LOGIN);
        assert_eq!(generated::play::clientbound::NAMES.len(), 124);
        assert_eq!(generated::play::serverbound::NAMES.len(), 58);
    }

    #[test]
    fn configuration_packet_ids_match_protocol_767_inventory() {
        use ids::{configuration_clientbound as cb, configuration_serverbound as sb};
        assert_eq!(
            [cb::COOKIE_REQUEST, cb::CUSTOM_PAYLOAD, cb::DISCONNECT,
             cb::FINISH_CONFIGURATION, cb::KEEP_ALIVE, cb::PING, cb::RESET_CHAT,
             cb::REGISTRY_DATA, cb::RESOURCE_PACK_POP, cb::RESOURCE_PACK_PUSH,
             cb::STORE_COOKIE, cb::TRANSFER, cb::UPDATE_ENABLED_FEATURES,
             cb::UPDATE_TAGS, cb::SELECT_KNOWN_PACKS, cb::CUSTOM_REPORT_DETAILS,
             cb::SERVER_LINKS],
            std::array::from_fn(|i| i as i32),
        );
        assert_eq!(
            [sb::CLIENT_INFORMATION, sb::COOKIE_RESPONSE, sb::CUSTOM_PAYLOAD,
             sb::FINISH_CONFIGURATION, sb::KEEP_ALIVE, sb::PONG, sb::RESOURCE_PACK,
             sb::SELECT_KNOWN_PACKS],
            std::array::from_fn(|i| i as i32),
        );
        assert_eq!(cb::NAMES.len(), 17);
        assert_eq!(sb::NAMES.len(), 8);
    }

    #[test]
    fn update_recipes_uses_the_registry_serializer_id_and_vanilla_codecs() {
        let serializers = VanillaRecipeSerializers;
        let packet = UpdateRecipes {
            recipes: vec![RecipeDefinition {
                id: "minecraft:test".into(),
                serializer: "minecraft:crafting_special_armordye".into(),
                body: vec![2],
            }],
        };
        let encoded = encode_update_recipes(&packet, &serializers).unwrap();
        let mut expected = Writer::new();
        expected.varint(ids::play_clientbound::UPDATE_RECIPES);
        expected.varint(1);
        expected.string("minecraft:test");
        expected.varint(2);
        expected.varint(2);
        assert_eq!(encoded, expected.data);
        assert_eq!(decode_update_recipes(&encoded[1..], &serializers).unwrap(), packet);
    }

    #[test]
    fn protocol_767_play_login_matches_the_1_21_1_wire_layout() {
        let packet = Login1211 {
            player_id: 42,
            hardcore: true,
            levels: vec!["minecraft:overworld".into(), "minecraft:the_nether".into()],
            max_players: 20,
            chunk_radius: 12,
            simulation_distance: 8,
            reduced_debug_info: false,
            show_death_screen: true,
            limited_crafting: false,
            spawn: SpawnInfo1211 {
                dimension_type: 0,
                dimension: "minecraft:overworld".into(),
                seed: 123456,
                game_type: 1,
                previous_game_type: Some(0),
                debug: false,
                flat: false,
                last_death: Some(GlobalPos1211 {
                    dimension: "minecraft:overworld".into(),
                    position: [10, 64, -3],
                }),
                portal_cooldown: 0,
            },
            enforces_secure_chat: true,
        };
        let mut encoded = Writer::new();
        Login1211::write(&packet, &mut encoded);
        assert_eq!(decode_play_login(&encoded.data).unwrap(), packet);
        assert_eq!(
            decode_play_clientbound(ids::play_clientbound::LOGIN, &encoded.data).unwrap(),
            MissingPacket::Login(packet.clone())
        );
        assert_eq!(
            decode_play_packet_1_21_1(ids::play_clientbound::LOGIN, &encoded.data).unwrap(),
            PlayPacket1211::Login(packet)
        );
        let mut trailing = encoded.data;
        trailing.push(0);
        assert!(decode_play_login(&trailing).is_err());
    }

    #[test]
    fn protocol_767_login_dispatch_uses_target_profile_payload() {
        let hello = login_packets::Hello {
            server_id: "server".into(),
            public_key: vec![1, 2, 3],
            challenge: vec![4, 5, 6],
            should_authenticate: true,
        };
        let mut hello_payload = Writer::new();
        login_packets::Hello::write(&hello, &mut hello_payload);
        assert_eq!(
            decode_login_packet_1_21_1(1, &hello_payload.data).unwrap(),
            LoginPacket1211::Shared(login_packets::Clientbound::Hello(hello))
        );

        let profile = GameProfilePacket {
            profile: GameProfile {
                uuid: 0x1234,
                name: "Nexora".into(),
                properties: vec![],
            },
            strict_error_handling: true,
        };
        let mut profile_payload = Writer::new();
        GameProfilePacket::write(&profile, &mut profile_payload);
        assert_eq!(
            decode_login_packet_1_21_1(
                ids::login_clientbound::GAME_PROFILE,
                &profile_payload.data
            )
            .unwrap(),
            LoginPacket1211::GameProfile(profile)
        );
    }

    #[test]
    fn protocol_767_client_information_omits_the_newer_particle_status_field() {
        let info = cfg::ClientInformation {
            language: "nl_nl".into(),
            view_distance: 12,
            chat_visibility: 1,
            chat_colors: true,
            model_customisation: 0x7f,
            main_hand: 1,
            text_filtering: false,
            allows_listing: true,
            particle_status: 2,
        };
        let encoded = encode_configuration_serverbound_1_21_1(
            cfg::Serverbound::ClientInformation(info),
        )
        .unwrap();
        let mut expected = Writer::new();
        expected.varint(ids::configuration_serverbound::CLIENT_INFORMATION);
        expected.string("nl_nl");
        expected.i8(12);
        expected.varint(1);
        expected.bool(true);
        expected.u8(0x7f);
        expected.varint(1);
        expected.bool(false);
        expected.bool(true);
        assert_eq!(encoded, expected.data);
        let mut decoded = Reader::new(&encoded);
        assert_eq!(decoded.varint().unwrap(), 0);
        assert_eq!(decoded.string(16).unwrap(), "nl_nl");
        assert_eq!(decoded.i8().unwrap(), 12);
        assert_eq!(decoded.varint().unwrap(), 1);
        assert!(decoded.bool().unwrap());
        assert_eq!(decoded.u8().unwrap(), 0x7f);
        assert_eq!(decoded.varint().unwrap(), 1);
        assert!(!decoded.bool().unwrap());
        assert!(decoded.bool().unwrap());
        assert_eq!(decoded.remaining(), 0);
    }

    #[test]
    fn protocol_767_configuration_dispatch_remaps_shifted_packet_ids() {
        let keep_alive = cfg::KeepAlive { id: 42 };
        let mut keep_alive_payload = Writer::new();
        cfg::KeepAlive::write(&keep_alive, &mut keep_alive_payload);
        assert_eq!(
            decode_configuration_packet_1_21_1(
                ids::configuration_clientbound::KEEP_ALIVE,
                &keep_alive_payload.data
            )
            .unwrap(),
            ConfigurationPacket1211::Shared(cfg::Clientbound::KeepAlive(keep_alive))
        );

        let cookie = cfg::StoreCookie {
            key: "proxy:state".into(),
            payload: vec![1, 2, 3],
        };
        let mut cookie_payload = Writer::new();
        cfg::StoreCookie::write(&cookie, &mut cookie_payload);
        assert_eq!(
            decode_configuration_packet_1_21_1(
                ids::configuration_clientbound::STORE_COOKIE,
                &cookie_payload.data
            )
            .unwrap(),
            ConfigurationPacket1211::Shared(cfg::Clientbound::StoreCookie(cookie))
        );

        let details = CustomReportDetails {
            details: vec![("server".into(), "1.21.1".into())],
        };
        let mut details_payload = Writer::new();
        CustomReportDetails::write(&details, &mut details_payload);
        assert_eq!(
            decode_configuration_packet_1_21_1(
                ids::configuration_clientbound::CUSTOM_REPORT_DETAILS,
                &details_payload.data
            )
            .unwrap(),
            ConfigurationPacket1211::CustomReportDetails(details)
        );

        let finish = cfg::Serverbound::FinishConfiguration(Empty);
        assert_eq!(
            encode_configuration_serverbound_1_21_1(finish.clone()).unwrap(),
            finish.encode()
        );
    }

    #[test]
    fn protocol_767_chunk_sections_are_promoted_to_the_active_palette_widths() {
        let block_values: Vec<i32> = (0..4096).map(|i| i % 512).collect();
        let biome_values: Vec<i32> = (0..64).map(|i| i % 9).collect();
        let blocks = Paletted::from_values(&block_values, Strategy::Blocks, 15);
        let biomes = Paletted::from_values(&biome_values, Strategy::Biomes, 6);
        let mut section_data = Writer::new();
        section_data.i16(12);
        blocks.write(&mut section_data);
        biomes.write(&mut section_data);

        let mut light = ActiveLightData {
            sky_mask: vec![0b0000_0100],
            block_mask: vec![0b0000_0001],
            empty_sky_mask: vec![],
            empty_block_mask: vec![],
            sky: vec![vec![0xF0; 2048]],
            block: vec![vec![0x0F; 2048]],
        };
        let mut packet = Writer::new();
        packet.i32(-3);
        packet.i32(7);
        packet.nbt(&Tag::Compound(Default::default()));
        packet.byte_array(&section_data.data);
        packet.varint(0);
        write_target_light(&light, &mut packet);

        let decoded = decode_level_chunk_with_light_1_21_1(&packet.data).unwrap();
        assert_eq!((decoded.x, decoded.z), (-3, 7));
        assert!(decoded.chunk.heightmaps.is_empty());
        let sections = decoded.chunk.sections(1, 16, 7).unwrap();
        assert_eq!(sections[0].non_empty, 12);
        assert_eq!(sections[0].states.bits, 16);
        assert_eq!(sections[0].biomes.bits, 7);
        assert_eq!(decoded.light.sky_mask[0], 0b0000_0100);
        assert_eq!(decoded.light.block_mask[0], 0b0000_0001);
        assert_eq!(decoded.light.sky[0], vec![0xF0; 2048]);

        assert_eq!(
            decode_play_packet_1_21_1(
                crate::packet_ids_1_21_1::play::clientbound::LEVEL_CHUNK_WITH_LIGHT,
                &packet.data
            )
            .unwrap(),
            PlayPacket1211::LevelChunkWithLight(decoded)
        );

        light.sky_mask.clear();
        let mut truncated = packet.data;
        truncated.pop();
        assert!(decode_level_chunk_with_light_1_21_1(&truncated).is_err());
    }

    #[test]
    fn protocol_767_relative_entity_move_packets_convert_ground_and_angles() {
        let position = MoveEntityPos1211 {
            entity_id: 5,
            delta: [10, -20, 30],
            on_ground: true,
        };
        let mut wire = Writer::new();
        MoveEntityPos1211::write(&position, &mut wire);
        assert_eq!(
            decode_move_entity_pos_1_21_1(&wire.data).unwrap(),
            play_packets::MoveEntityPos(play_packets::MoveEntity {
                id: 5,
                delta: Some(play_packets::VecDelta::Linear([10, -20, 30])),
                rotation: None,
                on_ground: true,
            })
        );

        let position_rotation = MoveEntityPosRot1211 {
            entity_id: 6,
            delta: [1, 2, 3],
            yaw: 64,
            pitch: -32,
            on_ground: false,
        };
        let mut wire = Writer::new();
        MoveEntityPosRot1211::write(&position_rotation, &mut wire);
        assert_eq!(
            decode_move_entity_pos_rot_1_21_1(&wire.data).unwrap(),
            play_packets::MoveEntityPosRot(play_packets::MoveEntity {
                id: 6,
                delta: Some(play_packets::VecDelta::Linear([1, 2, 3])),
                rotation: Some((64, -32)),
                on_ground: false,
            })
        );

        let rotation = MoveEntityRot1211 {
            entity_id: 7,
            yaw: 0,
            pitch: 32,
            on_ground: true,
        };
        let mut wire = Writer::new();
        MoveEntityRot1211::write(&rotation, &mut wire);
        assert_eq!(
            decode_move_entity_rot_1_21_1(&wire.data).unwrap(),
            play_packets::MoveEntityRot(play_packets::MoveEntity {
                id: 7,
                delta: None,
                rotation: Some((0, 32)),
                on_ground: true,
            })
        );
    }

    #[test]
    fn protocol_767_entity_velocity_and_teleport_use_target_payloads() {
        let velocity = EntityMotion1211 {
            entity_id: 9,
            velocity: [800, -1600, 4000],
        };
        let mut wire = Writer::new();
        EntityMotion1211::write(&velocity, &mut wire);
        assert_eq!(
            decode_entity_motion_1_21_1(&wire.data).unwrap(),
            play_packets::SetEntityMotion {
                id: 9,
                velocity: [0.1, -0.2, 0.5],
            }
        );

        let teleport = TeleportEntity1211 {
            entity_id: 17,
            position: [12.5, 70.0, -3.25],
            yaw: 64,
            pitch: -32,
            on_ground: true,
        };
        let mut wire = Writer::new();
        TeleportEntity1211::write(&teleport, &mut wire);
        assert_eq!(
            decode_teleport_entity_1_21_1(&wire.data).unwrap(),
            play_packets::TeleportEntity {
                id: 17,
                position: [12.5, 70.0, -3.25],
                delta: [0.0; 3],
                yaw: 90.0,
                pitch: -45.0,
                relatives: 0,
                on_ground: true,
            }
        );
    }

    #[test]
    fn protocol_767_initial_world_packet_codecs_use_the_target_layouts() {
        let position = PlayerPosition1211 {
            x: 12.5,
            y: 70.0,
            z: -3.25,
            yaw: 90.0,
            pitch: -10.0,
            relatives: 0b1_0011,
            teleport_id: 27,
        };
        let mut wire = Writer::new();
        PlayerPosition1211::write(&position, &mut wire);
        assert_eq!(
            decode_player_position_1_21_1(&wire.data).unwrap(),
            play_packets::PlayerPosition {
                teleport: 27,
                position: [12.5, 70.0, -3.25],
                delta: [0.0; 3],
                yaw: 90.0,
                pitch: -10.0,
                relatives: 0b1_0011,
            }
        );

        let difficulty = ChangeDifficulty1211 {
            difficulty: 2,
            locked: true,
        };
        let mut wire = Writer::new();
        ChangeDifficulty1211::write(&difficulty, &mut wire);
        assert_eq!(
            decode_change_difficulty_1_21_1(&wire.data).unwrap(),
            play_packets::ChangeDifficulty {
                difficulty: 2,
                locked: true,
            }
        );

        let spawn = SpawnPosition1211 {
            position: [100, 64, -100],
            angle: 45.0,
        };
        let mut wire = Writer::new();
        SpawnPosition1211::write(&spawn, &mut wire);
        assert_eq!(decode_spawn_position_1_21_1(&wire.data).unwrap(), spawn);
        assert!(decode_spawn_position_1_21_1(&wire.data[..wire.data.len() - 1]).is_err());
    }

    #[test]
    fn protocol_767_outgoing_movement_packets_drop_newer_collision_flags() {
        let packet = play_packets::Serverbound::MovePlayerPos(play_packets::MovePlayerPos {
            x: 1.5,
            y: 64.0,
            z: -2.5,
            flags: 3,
        });
        let encoded = encode_play_serverbound_1_21_1(packet).unwrap();
        let mut reader = Reader::new(&encoded);
        assert_eq!(
            reader.varint().unwrap(),
            crate::packet_ids_1_21_1::play::serverbound::NAMES
                .iter()
                .position(|name| *name == "move_player_pos")
                .unwrap() as i32
        );
        assert_eq!(reader.f64().unwrap(), 1.5);
        assert_eq!(reader.f64().unwrap(), 64.0);
        assert_eq!(reader.f64().unwrap(), -2.5);
        assert!(reader.bool().unwrap());
        assert_eq!(reader.remaining(), 0);

        let vehicle =
            play_packets::Serverbound::MoveVehicle(play_packets::MoveVehicleRequest {
                position: [1.0, 2.0, 3.0],
                yaw: 45.0,
                pitch: 10.0,
                on_ground: true,
            });
        let encoded = encode_play_serverbound_1_21_1(vehicle).unwrap();
        let mut reader = Reader::new(&encoded);
        assert_eq!(
            reader.varint().unwrap(),
            crate::packet_ids_1_21_1::play::serverbound::NAMES
                .iter()
                .position(|name| *name == "move_vehicle")
                .unwrap() as i32
        );
        assert_eq!(
            [reader.f64().unwrap(), reader.f64().unwrap(), reader.f64().unwrap()],
            [1.0, 2.0, 3.0]
        );
        assert_eq!(reader.f32().unwrap(), 45.0);
        assert_eq!(reader.f32().unwrap(), 10.0);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn protocol_767_teleport_ack_sends_only_the_teleport_id() {
        let packet = play_packets::Serverbound::AcceptTeleportation(
            play_packets::AcceptTeleportation {
                id: 44,
                x: 100.5,
                y: 64.0,
                z: -200.25,
                yaw: 90.0,
                pitch: 10.0,
            },
        );
        let encoded = encode_play_serverbound_1_21_1(packet).unwrap();
        let mut reader = Reader::new(&encoded);
        assert_eq!(
            reader.varint().unwrap(),
            crate::packet_ids_1_21_1::play::serverbound::NAMES
                .iter()
                .position(|name| *name == "accept_teleportation")
                .unwrap() as i32
        );
        assert_eq!(reader.varint().unwrap(), 44);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn protocol_767_loaded_and_chunk_batch_packets_keep_their_target_layouts() {
        let loaded = encode_play_serverbound_1_21_1(
            play_packets::Serverbound::PlayerLoaded(Empty),
        )
        .unwrap();
        let mut reader = Reader::new(&loaded);
        assert_eq!(
            reader.varint().unwrap(),
            crate::packet_ids_1_21_1::play::serverbound::NAMES
                .iter()
                .position(|name| *name == "player_loaded")
                .unwrap() as i32
        );
        assert_eq!(reader.remaining(), 0);

        let batch = encode_play_serverbound_1_21_1(
            play_packets::Serverbound::ChunkBatchReceived(
                play_packets::ChunkBatchReceived {
                    chunks_per_tick: 48.0,
                },
            ),
        )
        .unwrap();
        let mut reader = Reader::new(&batch);
        assert_eq!(
            reader.varint().unwrap(),
            crate::packet_ids_1_21_1::play::serverbound::NAMES
                .iter()
                .position(|name| *name == "chunk_batch_received")
                .unwrap() as i32
        );
        assert_eq!(reader.f32().unwrap(), 48.0);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn protocol_767_block_place_drops_the_newer_world_border_flag() {
        let packet = play_packets::Serverbound::UseItemOn(play_packets::UseItemOn {
            hand: 1,
            pos: [10, 64, -20],
            face: 5,
            cursor: [0.25, 0.5, 0.75],
            inside: true,
            world_border: true,
            sequence: 17,
        });
        let encoded = encode_play_serverbound_1_21_1(packet).unwrap();
        let mut reader = Reader::new(&encoded);
        assert_eq!(
            reader.varint().unwrap(),
            crate::packet_ids_1_21_1::play::serverbound::NAMES
                .iter()
                .position(|name| *name == "use_item_on")
                .unwrap() as i32
        );
        assert_eq!(reader.varint().unwrap(), 1);
        assert_eq!(unpack_block_pos(reader.i64().unwrap()), [10, 64, -20]);
        assert_eq!(reader.varint().unwrap(), 5);
        assert_eq!(
            [reader.f32().unwrap(), reader.f32().unwrap(), reader.f32().unwrap()],
            [0.25, 0.5, 0.75]
        );
        assert!(reader.bool().unwrap());
        assert_eq!(reader.varint().unwrap(), 17);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn protocol_767_core_play_packets_use_target_ids_and_shared_payload_codecs() {
        let keep_alive = cfg::KeepAlive { id: 0x102030405060708 };
        let mut payload = Writer::new();
        cfg::KeepAlive::write(&keep_alive, &mut payload);
        assert_eq!(
            decode_play_packet_1_21_1(ids::play_clientbound::KEEP_ALIVE, &payload.data).unwrap(),
            PlayPacket1211::KeepAlive(keep_alive)
        );

        let cookie = cfg::CookieRequest {
            key: "proxy:session".into(),
        };
        let mut payload = Writer::new();
        cfg::CookieRequest::write(&cookie, &mut payload);
        assert_eq!(
            decode_play_packet_1_21_1(ids::play_clientbound::COOKIE_REQUEST, &payload.data)
                .unwrap(),
            PlayPacket1211::CookieRequest(cookie)
        );

        assert_eq!(
            decode_play_packet_1_21_1(ids::play_clientbound::BUNDLE_DELIMITER, &[]).unwrap(),
            PlayPacket1211::BundleDelimiter(Empty)
        );
        assert_eq!(
            decode_play_packet_1_21_1(ids::play_clientbound::START_CONFIGURATION, &[]).unwrap(),
            PlayPacket1211::StartConfiguration(Empty)
        );
        assert!(decode_play_packet_1_21_1(
            crate::packet_ids_1_21_1::play::clientbound::LEVEL_CHUNK_WITH_LIGHT,
            &[]
        )
        .is_err());
    }

    #[test]
    fn simple_missing_packets_round_trip() {
        round_trip(AddExperienceOrb { entity_id: 9, x: 1.25, y: 64.0, z: -3.5, value: 7 });
        round_trip(DebugSample { samples: vec![1, -2, i64::MAX], sample_type: 2 });
        round_trip(HorseScreenOpen { container_id: 3, size: 5, entity_id: 900 });
        round_trip(PlayerCombatEnd { duration: 120 });
        round_trip(SelectAdvancementsTab { tab: Some("minecraft:story/root".into()) });
        round_trip(SelectAdvancementsTab { tab: None });
        round_trip(SetCarriedItem { slot: 8 });
        round_trip(CommandSuggestionRequest { transaction_id: 4, text: "/time set".into() });
        round_trip(DebugSampleSubscription { sample_type: 1 });
        round_trip(PickItem { slot: 36 });
        round_trip(RecipeBookSeenRecipe { recipe: "minecraft:oak_planks".into() });
        round_trip(SeenAdvancements { action: 0, tab: Some("minecraft:story/root".into()) });
        round_trip(SeenAdvancements { action: 1, tab: None });
        round_trip(Swing { hand: 1 });
        round_trip(CustomReportDetails { details: vec![("server".into(), "nexora".into())] });
        round_trip(CommandSuggestions {
            transaction_id: 8, start: 1, length: 4,
            matches: vec![Suggestion { text: "help".into(), tooltip: Some(Tag::String("Help".into())) }],
        });
        round_trip(RecipePacket {
            action: 0,
            recipes: vec!["minecraft:stone".into()],
            highlights: vec!["minecraft:oak_planks".into()],
            book_settings: RecipeBookSettings { books: [(true, false), (false, true), (true, true), (false, false)] },
        });
    }

    #[test]
    fn play_decoders_reject_trailing_bytes_and_use_target_ids() {
        let mut w = Writer::new();
        PickItem::write(&PickItem { slot: 5 }, &mut w);
        assert_eq!(decode_play_serverbound(ids::play_serverbound::PICK_ITEM, &w.data), Ok(MissingPacket::PickItem(PickItem { slot: 5 })));
        w.data.push(0);
        assert!(decode_play_serverbound(ids::play_serverbound::PICK_ITEM, &w.data).is_err());
        assert_eq!(ids::play_clientbound::COMMAND_SUGGESTIONS, 16);
        assert_eq!(ids::play_clientbound::BUNDLE_DELIMITER, 0);
        assert_eq!(ids::play_clientbound::KEEP_ALIVE, 38);
        assert_eq!(ids::play_clientbound::LOGIN, 43);
        assert_eq!(ids::play_clientbound::START_CONFIGURATION, 105);
        assert_eq!(ids::play_clientbound::SERVER_LINKS, 123);
        assert_eq!(ids::configuration_clientbound::CUSTOM_REPORT_DETAILS, 15);
    }
}


pub fn decode_update_recipes(
    payload: &[u8],
    serializers: &impl RecipeSerializer,
) -> Result<UpdateRecipes> {
    decode_exact(payload, |r| UpdateRecipes::read_with(r, serializers))
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn game_profile_packet_keeps_the_1211_strict_error_flag() {
        let value = GameProfilePacket {
            profile: GameProfile {
                uuid: 0x1234,
                name: "Nexora".into(),
                properties: vec![],
            },
            strict_error_handling: true,
        };
        let mut writer = Writer::new();
        GameProfilePacket::write(&value, &mut writer);
        let decoded = decode_exact(&writer.data, GameProfilePacket::read).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn empty_combat_enter_has_no_payload() {
        assert_eq!(
            decode_play_clientbound(ids::play_clientbound::PLAYER_COMBAT_ENTER, &[]).unwrap(),
            MissingPacket::PlayerCombatEnter(Empty)
        );
    }
}


pub fn encode_play_clientbound(packet: &MissingPacket) -> Result<Vec<u8>> {
    let (id, write): (i32, Box<dyn FnOnce(&mut Writer) + '_>) = match packet {
        MissingPacket::AddExperienceOrb(v) => (ids::play_clientbound::ADD_EXPERIENCE_ORB, Box::new(|w| AddExperienceOrb::write(v, w))),
        MissingPacket::CommandSuggestions(v) => (ids::play_clientbound::COMMAND_SUGGESTIONS, Box::new(|w| CommandSuggestions::write(v, w))),
        MissingPacket::CustomReportDetails(v) => (ids::play_clientbound::CUSTOM_REPORT_DETAILS, Box::new(|w| CustomReportDetails::write(v, w))),
        MissingPacket::DebugSample(v) => (ids::play_clientbound::DEBUG_SAMPLE, Box::new(|w| DebugSample::write(v, w))),
        MissingPacket::HorseScreenOpen(v) => (ids::play_clientbound::HORSE_SCREEN_OPEN, Box::new(|w| HorseScreenOpen::write(v, w))),
        MissingPacket::PlayerCombatEnd(v) => (ids::play_clientbound::PLAYER_COMBAT_END, Box::new(|w| PlayerCombatEnd::write(v, w))),
        MissingPacket::PlayerCombatEnter(v) => (ids::play_clientbound::PLAYER_COMBAT_ENTER, Box::new(|w| Empty::write(v, w))),
        MissingPacket::Recipe(v) => (ids::play_clientbound::RECIPE, Box::new(|w| RecipePacket::write(v, w))),
        MissingPacket::SelectAdvancementsTab(v) => (ids::play_clientbound::SELECT_ADVANCEMENTS_TAB, Box::new(|w| SelectAdvancementsTab::write(v, w))),
        MissingPacket::SetCarriedItem(v) => (ids::play_clientbound::SET_CARRIED_ITEM, Box::new(|w| SetCarriedItem::write(v, w))),
        _ => return Err(Error::new("packet is not a 1.21.1 clientbound play packet")),
    };
    let mut w = Writer::new();
    w.varint(id);
    write(&mut w);
    Ok(w.data)
}

pub fn encode_play_serverbound(packet: &MissingPacket) -> Result<Vec<u8>> {
    let (id, write): (i32, Box<dyn FnOnce(&mut Writer) + '_>) = match packet {
        MissingPacket::CommandSuggestionRequest(v) => (ids::play_serverbound::COMMAND_SUGGESTION, Box::new(|w| CommandSuggestionRequest::write(v, w))),
        MissingPacket::DebugSampleSubscription(v) => (ids::play_serverbound::DEBUG_SAMPLE_SUBSCRIPTION, Box::new(|w| DebugSampleSubscription::write(v, w))),
        MissingPacket::PickItem(v) => (ids::play_serverbound::PICK_ITEM, Box::new(|w| PickItem::write(v, w))),
        MissingPacket::RecipeBookSeenRecipe(v) => (ids::play_serverbound::RECIPE_BOOK_SEEN_RECIPE, Box::new(|w| RecipeBookSeenRecipe::write(v, w))),
        MissingPacket::SeenAdvancements(v) => (ids::play_serverbound::SEEN_ADVANCEMENTS, Box::new(|w| SeenAdvancements::write(v, w))),
        MissingPacket::Swing(v) => (ids::play_serverbound::SWING, Box::new(|w| Swing::write(v, w))),
        _ => return Err(Error::new("packet is not a 1.21.1 serverbound play packet")),
    };
    let mut w = Writer::new();
    w.varint(id);
    write(&mut w);
    Ok(w.data)
}


pub fn encode_update_recipes(
    packet: &UpdateRecipes,
    serializers: &impl RecipeSerializer,
) -> Result<Vec<u8>> {
    let mut w = Writer::new();
    w.varint(ids::play_clientbound::UPDATE_RECIPES);
    packet.write_with(&mut w, serializers)?;
    Ok(w.data)
}


/// Configuration-state packet IDs and codecs for protocol 767.
pub mod configuration {
    use super::{CustomReportDetails, decode_exact, ids};
    use crate::{
        Error, Result,
        buf::{Codec, Reader, Writer},
        packets::{Empty, configuration as shared},
    };

    /// Decode a protocol 767 clientbound configuration packet.
    /// Custom report details are decoded and discarded; they are not collected.
    pub fn decode_clientbound(id: i32, payload: &[u8]) -> Result<Option<shared::Clientbound>> {
        let mut r = Reader::new(payload);
        let packet = match id {
            ids::configuration_clientbound::COOKIE_REQUEST => Some(shared::Clientbound::CookieRequest(shared::CookieRequest::read(&mut r)?)),
            ids::configuration_clientbound::CUSTOM_PAYLOAD => Some(shared::Clientbound::CustomPayload(shared::CustomPayload::read(&mut r)?)),
            ids::configuration_clientbound::DISCONNECT => Some(shared::Clientbound::Disconnect(shared::Disconnect::read(&mut r)?)),
            ids::configuration_clientbound::FINISH_CONFIGURATION => Some(shared::Clientbound::FinishConfiguration(Empty::read(&mut r)?)),
            ids::configuration_clientbound::KEEP_ALIVE => Some(shared::Clientbound::KeepAlive(shared::KeepAlive::read(&mut r)?)),
            ids::configuration_clientbound::PING => Some(shared::Clientbound::Ping(shared::Ping::read(&mut r)?)),
            ids::configuration_clientbound::RESET_CHAT => Some(shared::Clientbound::ResetChat(Empty::read(&mut r)?)),
            ids::configuration_clientbound::REGISTRY_DATA => Some(shared::Clientbound::RegistryData(shared::RegistryData::read(&mut r)?)),
            ids::configuration_clientbound::RESOURCE_PACK_POP => Some(shared::Clientbound::ResourcePackPop(shared::ResourcePackPop::read(&mut r)?)),
            ids::configuration_clientbound::RESOURCE_PACK_PUSH => Some(shared::Clientbound::ResourcePackPush(shared::ResourcePackPush::read(&mut r)?)),
            ids::configuration_clientbound::STORE_COOKIE => Some(shared::Clientbound::StoreCookie(shared::StoreCookie::read(&mut r)?)),
            ids::configuration_clientbound::TRANSFER => Some(shared::Clientbound::Transfer(shared::Transfer::read(&mut r)?)),
            ids::configuration_clientbound::UPDATE_ENABLED_FEATURES => Some(shared::Clientbound::UpdateEnabledFeatures(shared::UpdateEnabledFeatures::read(&mut r)?)),
            ids::configuration_clientbound::UPDATE_TAGS => Some(shared::Clientbound::UpdateTags(shared::UpdateTags::read(&mut r)?)),
            ids::configuration_clientbound::SELECT_KNOWN_PACKS => Some(shared::Clientbound::SelectKnownPacks(shared::SelectKnownPacks::read(&mut r)?)),
            ids::configuration_clientbound::CUSTOM_REPORT_DETAILS => {
                decode_exact(payload, CustomReportDetails::read)?;
                return Ok(None);
            }
            ids::configuration_clientbound::SERVER_LINKS => Some(shared::Clientbound::ServerLinks(shared::ServerLinks::read(&mut r)?)),
            other => return Err(Error::new(format!("unsupported protocol 767 configuration clientbound packet {other}"))),
        };
        if r.remaining() != 0 {
            return Err(Error::new(format!("protocol 767 configuration packet {id} has {} trailing bytes", r.remaining())));
        }
        Ok(packet)
    }

    /// Encode a shared configuration packet with protocol 767's packet ID.
    pub fn encode_serverbound(packet: &shared::Serverbound) -> Result<Vec<u8>> {
        let mut payload = Writer::new();
        let id = match packet {
            shared::Serverbound::ClientInformation(p) => { shared::ClientInformation::write(p, &mut payload); ids::configuration_serverbound::CLIENT_INFORMATION }
            shared::Serverbound::CookieResponse(p) => { shared::CookieResponse::write(p, &mut payload); ids::configuration_serverbound::COOKIE_RESPONSE }
            shared::Serverbound::CustomPayload(p) => { shared::CustomPayload::write(p, &mut payload); ids::configuration_serverbound::CUSTOM_PAYLOAD }
            shared::Serverbound::FinishConfiguration(p) => { Empty::write(p, &mut payload); ids::configuration_serverbound::FINISH_CONFIGURATION }
            shared::Serverbound::KeepAlive(p) => { shared::KeepAlive::write(p, &mut payload); ids::configuration_serverbound::KEEP_ALIVE }
            shared::Serverbound::Pong(p) => { shared::Ping::write(p, &mut payload); ids::configuration_serverbound::PONG }
            shared::Serverbound::ResourcePack(p) => { shared::ResourcePackResponse::write(p, &mut payload); ids::configuration_serverbound::RESOURCE_PACK }
            shared::Serverbound::SelectKnownPacks(p) => { shared::SelectKnownPacks::write(p, &mut payload); ids::configuration_serverbound::SELECT_KNOWN_PACKS }
            _ => return Err(Error::new("serverbound packet is not part of protocol 767 configuration state")),
        };
        let mut out = Writer::new();
        out.varint(id);
        out.bytes(&payload.data);
        Ok(out.data)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn configuration_packet_ids_are_protocol_767_scoped() {
            let finish = encode_serverbound(&shared::Serverbound::FinishConfiguration(Empty)).unwrap();
            assert_eq!(finish, vec![3]);
            let ping = encode_serverbound(&shared::Serverbound::Pong(shared::Ping { id: 42 })).unwrap();
            assert_eq!(ping[0], 5);
            assert_eq!(decode_clientbound(3, &[]).unwrap(), Some(shared::Clientbound::FinishConfiguration(Empty)));
            assert!(decode_clientbound(17, &[]).is_err());
        }

        #[test]
        fn unsupported_newer_configuration_packets_return_errors() {
            let packet = shared::Serverbound::AcceptCodeOfConduct(Empty);
            assert!(encode_serverbound(&packet).is_err());
        }

        #[test]
        fn custom_report_details_are_consumed_without_being_retained() {
            let mut writer = Writer::new();
            CustomReportDetails::write(
                &CustomReportDetails { details: vec![("key".into(), "value".into())] },
                &mut writer,
            );
            assert_eq!(decode_clientbound(15, &writer.data).unwrap(), None);
        }
    }
}

/// Login-state codecs for protocol 767. The packet IDs happen to match the
/// current shared login IDs, but the successful-login payload does not:
/// 1.21.1 sends GameProfilePacket (including strictErrorHandling), not the
/// newer LoginFinished structure with a session UUID.
pub mod login {
    use super::{GameProfilePacket, PROTOCOL_VERSION};
    use crate::{
        Result,
        buf::{Codec, Reader, Writer},
        packets::{Empty, login as shared},
    };

    pub const DISCONNECT: i32 = 0;
    pub const HELLO: i32 = 1;
    pub const GAME_PROFILE: i32 = 2;
    pub const COMPRESSION: i32 = 3;
    pub const CUSTOM_QUERY: i32 = 4;
    pub const COOKIE_REQUEST: i32 = 5;

    #[derive(Clone, Debug, PartialEq)]
    pub enum Clientbound {
        Disconnect(shared::Disconnect),
        Hello(shared::Hello),
        GameProfile(GameProfilePacket),
        Compression(shared::Compression),
        CustomQuery(shared::CustomQuery),
        CookieRequest(shared::CookieRequest),
        Unknown(i32, Vec<u8>),
    }

    impl Clientbound {
        pub fn decode(id: i32, payload: &[u8]) -> Result<Self> {
            let mut r = Reader::new(payload);
            let packet = match id {
                DISCONNECT => Self::Disconnect(shared::Disconnect::read(&mut r)?),
                HELLO => Self::Hello(shared::Hello::read(&mut r)?),
                GAME_PROFILE => Self::GameProfile(GameProfilePacket::read(&mut r)?),
                COMPRESSION => Self::Compression(shared::Compression::read(&mut r)?),
                CUSTOM_QUERY => Self::CustomQuery(shared::CustomQuery::read(&mut r)?),
                COOKIE_REQUEST => Self::CookieRequest(shared::CookieRequest::read(&mut r)?),
                _ => return Ok(Self::Unknown(id, payload.to_vec())),
            };
            if r.remaining() != 0 {
                return Err(crate::Error::new(format!(
                    "protocol {PROTOCOL_VERSION} login packet {id} has {} trailing bytes",
                    r.remaining()
                )));
            }
            Ok(packet)
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum Serverbound {
        Hello(shared::ServerHello),
        Key(shared::Key),
        CustomQueryAnswer(shared::CustomQueryAnswer),
        LoginAcknowledged(Empty),
        CookieResponse(shared::CookieResponse),
    }

    impl Serverbound {
        pub fn encode(&self) -> Vec<u8> {
            let mut w = Writer::new();
            let id = match self {
                Self::Hello(p) => { shared::ServerHello::write(p, &mut w); 0 }
                Self::Key(p) => { shared::Key::write(p, &mut w); 1 }
                Self::CustomQueryAnswer(p) => { shared::CustomQueryAnswer::write(p, &mut w); 2 }
                Self::LoginAcknowledged(p) => { Empty::write(p, &mut w); 3 }
                Self::CookieResponse(p) => { shared::CookieResponse::write(p, &mut w); 4 }
            };
            let mut body = Writer::new();
            body.varint(id);
            body.bytes(&w.data);
            body.data
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn profile_packet_uses_the_1211_payload_and_login_ids() {
            let profile = GameProfilePacket {
                profile: shared::GameProfile {
                    uuid: 7,
                    name: "Nexora".into(),
                    properties: vec![],
                },
                strict_error_handling: true,
            };
            let mut payload = Writer::new();
            GameProfilePacket::write(&profile, &mut payload);
            assert_eq!(
                Clientbound::decode(GAME_PROFILE, &payload.data).unwrap(),
                Clientbound::GameProfile(profile)
            );
            assert!(Clientbound::decode(GAME_PROFILE, &[0]).is_err());
        }

        #[test]
        fn serverbound_login_ids_and_payloads_are_version_scoped() {
            let packets = [
                Serverbound::LoginAcknowledged(Empty),
                Serverbound::Hello(shared::ServerHello { name: "Nexora".into(), uuid: 7 }),
            ];
            assert_eq!(packets[0].encode(), vec![3]);
            assert_eq!(packets[1].encode()[0], 0);
        }
    }
}


#[cfg(test)]
mod live_inventory_tests {
    use super::*;
    use crate::buf::Codec;

    #[test]
    fn play_dispatch_uses_protocol_767_inventory_slot_codecs() {
        let packet = inventory::ContainerSetContent {
            container: 3,
            state: 12,
            items: vec![Some(crate::item::Stack::new(5, 2)), None],
            carried: None,
        };
        let mut payload = Writer::new();
        inventory::ContainerSetContent::write(&packet, &mut payload);

        assert_eq!(
            decode_play_packet_1_21_1(
                crate::packet_ids_1_21_1::play::clientbound::CONTAINER_SET_CONTENT,
                &payload.data,
            )
            .unwrap(),
            PlayPacket1211::Inventory(inventory::Clientbound::SetContent(packet))
        );
    }

    #[test]
    fn live_inventory_serverbound_packets_use_protocol_767_slot_codecs() {
        let click = inventory::ContainerClick {
            container: 2,
            state: 7,
            slot: 4,
            button: 1,
            mode: 0,
            changed: vec![inventory::ChangedSlot {
                slot: 4,
                item: Some(crate::item::Hashed {
                    item: 5,
                    count: 2,
                    added: vec![],
                    removed: vec![],
                }),
            }],
            carried: None,
        };
        let encoded = encode_play_serverbound_1_21_1(
            play_packets::Serverbound::ContainerClick(play_packets::ContainerClick {
                container: click.container,
                state: click.state,
                slot: click.slot,
                button: click.button,
                input: click.mode,
                changed: click
                    .changed
                    .iter()
                    .map(|change| (change.slot, change.item.clone()))
                    .collect(),
                carried: click.carried.clone(),
            }),
        )
        .unwrap();
        let mut reader = Reader::new(&encoded);
        assert_eq!(
            reader.varint().unwrap(),
            crate::packet_ids_1_21_1::play::serverbound::CONTAINER_CLICK
        );
        assert_eq!(inventory::ContainerClick::read(&mut reader).unwrap(), click);
        assert_eq!(reader.remaining(), 0);

        let creative = crate::item::Stack::new(9, 1);
        let encoded = encode_play_serverbound_1_21_1(
            play_packets::Serverbound::SetCreativeModeSlot(
                play_packets::SetCreativeModeSlot {
                    slot: 36,
                    item: Some(creative.clone()),
                },
            ),
        )
        .unwrap();
        let mut reader = Reader::new(&encoded);
        assert_eq!(
            reader.varint().unwrap(),
            crate::packet_ids_1_21_1::play::serverbound::SET_CREATIVE_MODE_SLOT
        );
        assert_eq!(
            inventory::SetCreativeModeSlot::read(&mut reader).unwrap(),
            inventory::SetCreativeModeSlot {
                slot: 36,
                item: Some(creative),
            }
        );
        assert_eq!(reader.remaining(), 0);
    }
}
