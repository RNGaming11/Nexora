//! Minecraft Java Edition 1.21.1 (protocol 767) packet codecs that must
//! not be aliased to the 26.3 packet definitions in this crate.
//!
//! This module is deliberately version-scoped. It does not change the active
//! protocol IDs or the shared client connection. Recipe definitions use an
//! explicit serializer registry because their payloads are registry-driven.
use crate::{
    Error, Result,
    buf::{Codec, Reader, Writer},
    packets::{Empty, login::GameProfile, recipes::RecipeBookSettings},
};
use horizon_world::nbt::Tag;

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
            let serializer = r.string(32767)?;
            let body = serializers.read_recipe(&serializer, &id, r)?;
            recipes.push(RecipeDefinition { id, serializer, body });
        }
        Ok(Self { recipes })
    }
    pub fn write_with(&self, w: &mut Writer, serializers: &impl RecipeSerializer) -> Result<()> {
        w.varint(self.recipes.len() as i32);
        for recipe in &self.recipes {
            w.string(&recipe.id);
            w.string(&recipe.serializer);
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


/// Decode one of the missing 1.21.1 clientbound play packets whose payload
/// codec is fully represented above. Unknown IDs remain the caller's concern.
pub fn decode_play_clientbound(id: i32, payload: &[u8]) -> Result<MissingPacket> {
    let mut r = Reader::new(payload);
    let packet = match id {
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
    pub fn encode_serverbound(packet: &shared::Serverbound) -> Vec<u8> {
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
            _ => unreachable!("packet does not exist in protocol 767 configuration state"),
        };
        let mut out = Writer::new();
        out.varint(id);
        out.bytes(&payload.data);
        out.data
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn configuration_packet_ids_are_protocol_767_scoped() {
            let finish = encode_serverbound(&shared::Serverbound::FinishConfiguration(Empty));
            assert_eq!(finish, vec![3]);
            let ping = encode_serverbound(&shared::Serverbound::Pong(shared::Ping { id: 42 }));
            assert_eq!(ping[0], 5);
            assert_eq!(decode_clientbound(3, &[]).unwrap(), Some(shared::Clientbound::FinishConfiguration(Empty)));
            assert!(decode_clientbound(17, &[]).is_err());
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


/// Configuration-state packet dispatch for protocol 767. The payload codecs
/// are shared where their 1.21.1 wire layouts match; IDs are never taken from
/// the active 26.3 inventory.
pub mod configuration {
    use super::{CustomReportDetails, ids};
    use crate::{
        Error, Result,
        buf::{Codec, Reader, Writer},
        packets::{Empty, configuration as shared},
    };

    #[derive(Clone, Debug, PartialEq)]
    pub enum Clientbound {
        CookieRequest(shared::CookieRequest),
        CustomPayload(shared::CustomPayload),
        Disconnect(shared::Disconnect),
        FinishConfiguration(Empty),
        KeepAlive(shared::KeepAlive),
        Ping(shared::Ping),
        ResetChat(Empty),
        RegistryData(shared::RegistryData),
        ResourcePackPop(shared::ResourcePackPop),
        ResourcePackPush(shared::ResourcePackPush),
        StoreCookie(shared::StoreCookie),
        Transfer(shared::Transfer),
        UpdateEnabledFeatures(shared::UpdateEnabledFeatures),
        UpdateTags(shared::UpdateTags),
        SelectKnownPacks(shared::SelectKnownPacks),
        CustomReportDetails(CustomReportDetails),
        ServerLinks(shared::ServerLinks),
        Unknown(i32, Vec<u8>),
    }

    impl Clientbound {
        pub fn decode(id: i32, payload: &[u8]) -> Result<Self> {
            let mut r = Reader::new(payload);
            let packet = match id {
                ids::configuration_clientbound::COOKIE_REQUEST => Self::CookieRequest(shared::CookieRequest::read(&mut r)?),
                ids::configuration_clientbound::CUSTOM_PAYLOAD => Self::CustomPayload(shared::CustomPayload::read(&mut r)?),
                ids::configuration_clientbound::DISCONNECT => Self::Disconnect(shared::Disconnect::read(&mut r)?),
                ids::configuration_clientbound::FINISH_CONFIGURATION => Self::FinishConfiguration(Empty::read(&mut r)?),
                ids::configuration_clientbound::KEEP_ALIVE => Self::KeepAlive(shared::KeepAlive::read(&mut r)?),
                ids::configuration_clientbound::PING => Self::Ping(shared::Ping::read(&mut r)?),
                ids::configuration_clientbound::RESET_CHAT => Self::ResetChat(Empty::read(&mut r)?),
                ids::configuration_clientbound::REGISTRY_DATA => Self::RegistryData(shared::RegistryData::read(&mut r)?),
                ids::configuration_clientbound::RESOURCE_PACK_POP => Self::ResourcePackPop(shared::ResourcePackPop::read(&mut r)?),
                ids::configuration_clientbound::RESOURCE_PACK_PUSH => Self::ResourcePackPush(shared::ResourcePackPush::read(&mut r)?),
                ids::configuration_clientbound::STORE_COOKIE => Self::StoreCookie(shared::StoreCookie::read(&mut r)?),
                ids::configuration_clientbound::TRANSFER => Self::Transfer(shared::Transfer::read(&mut r)?),
                ids::configuration_clientbound::UPDATE_ENABLED_FEATURES => Self::UpdateEnabledFeatures(shared::UpdateEnabledFeatures::read(&mut r)?),
                ids::configuration_clientbound::UPDATE_TAGS => Self::UpdateTags(shared::UpdateTags::read(&mut r)?),
                ids::configuration_clientbound::SELECT_KNOWN_PACKS => Self::SelectKnownPacks(shared::SelectKnownPacks::read(&mut r)?),
                ids::configuration_clientbound::CUSTOM_REPORT_DETAILS => Self::CustomReportDetails(CustomReportDetails::read(&mut r)?),
                ids::configuration_clientbound::SERVER_LINKS => Self::ServerLinks(shared::ServerLinks::read(&mut r)?),
                _ => return Ok(Self::Unknown(id, payload.to_vec())),
            };
            if r.remaining() != 0 {
                return Err(Error::new(format!("1.21.1 configuration packet {id} has {} trailing bytes", r.remaining())));
            }
            Ok(packet)
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum Serverbound {
        ClientInformation(shared::ClientInformation),
        CookieResponse(shared::CookieResponse),
        CustomPayload(shared::CustomPayload),
        FinishConfiguration(Empty),
        KeepAlive(shared::KeepAlive),
        Pong(shared::Ping),
        ResourcePack(shared::ResourcePackResponse),
        SelectKnownPacks(shared::SelectKnownPacks),
    }

    impl Serverbound {
        pub fn encode(&self) -> Vec<u8> {
            let mut payload = Writer::new();
            let id = match self {
                Self::ClientInformation(v) => { shared::ClientInformation::write(v, &mut payload); ids::configuration_serverbound::CLIENT_INFORMATION }
                Self::CookieResponse(v) => { shared::CookieResponse::write(v, &mut payload); ids::configuration_serverbound::COOKIE_RESPONSE }
                Self::CustomPayload(v) => { shared::CustomPayload::write(v, &mut payload); ids::configuration_serverbound::CUSTOM_PAYLOAD }
                Self::FinishConfiguration(v) => { Empty::write(v, &mut payload); ids::configuration_serverbound::FINISH_CONFIGURATION }
                Self::KeepAlive(v) => { shared::KeepAlive::write(v, &mut payload); ids::configuration_serverbound::KEEP_ALIVE }
                Self::Pong(v) => { shared::Ping::write(v, &mut payload); ids::configuration_serverbound::PONG }
                Self::ResourcePack(v) => { shared::ResourcePackResponse::write(v, &mut payload); ids::configuration_serverbound::RESOURCE_PACK }
                Self::SelectKnownPacks(v) => { shared::SelectKnownPacks::write(v, &mut payload); ids::configuration_serverbound::SELECT_KNOWN_PACKS }
            };
            let mut body = Writer::new();
            body.varint(id);
            body.bytes(&payload.data);
            body.data
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn configuration_dispatch_uses_protocol_767_ids() {
            let keep_alive = Clientbound::decode(
                ids::configuration_clientbound::KEEP_ALIVE,
                &[42],
            ).unwrap();
            assert_eq!(keep_alive, Clientbound::KeepAlive(shared::KeepAlive { id: 42 }));
            let store_cookie = Serverbound::SelectKnownPacks(shared::SelectKnownPacks { packs: vec![] }).encode();
            assert_eq!(store_cookie, vec![7, 0]);
            let finish = Serverbound::FinishConfiguration(Empty).encode();
            assert_eq!(finish, vec![3]);
        }

        #[test]
        fn configuration_dispatch_rejects_trailing_payload_bytes() {
            assert!(Clientbound::decode(
                ids::configuration_clientbound::FINISH_CONFIGURATION,
                &[0],
            ).is_err());
        }
    }
}
