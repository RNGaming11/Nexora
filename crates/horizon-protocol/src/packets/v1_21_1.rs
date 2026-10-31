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
    entity_id: i32 => crate::buf::VarInt,
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
    pub mod configuration_clientbound { pub const CUSTOM_REPORT_DETAILS: i32 = 16; }
    pub mod login_clientbound { pub const GAME_PROFILE: i32 = 2; }
    pub mod play_clientbound {
        pub const ADD_EXPERIENCE_ORB: i32 = 2;
        pub const COMMAND_SUGGESTIONS: i32 = 15;
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
