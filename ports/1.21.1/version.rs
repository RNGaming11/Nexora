//! Minecraft Java Edition 1.21.1 version facts, kept separate from the active workspace target.
//! Values are pinned in ports/1.21.1/target.json and checked by tests.

/// The version identifier in the client jar's version.json.
pub const VERSION_ID: &str = "1.21.1";
/// The display version reported by the game.
pub const VANILLA_VERSION: &str = "1.21.1";
/// Official release timestamp from Mojang's version manifest.
pub const RELEASE_TIME: &str = "2024-08-08T12:24:45+00:00";
/// Mojang asset index used by this release.
pub const ASSET_INDEX: &str = "17";
/// SharedConstants.getProtocolVersion().
pub const PROTOCOL_VERSION: i32 = 767;
/// WorldVersion.dataVersion().
pub const DATA_VERSION: i32 = 3955;
/// Client resource pack format (1.21.1 uses a single integer).
pub const RESOURCE_PACK_FORMAT: (i64, i64) = (34, 0);
/// Server data pack format (1.21.1 uses a single integer).
pub const DATA_PACK_FORMAT: (i64, i64) = (48, 0);
/// Java major version required by the client and server.
pub const JAVA_VERSION: u32 = 21;
/// The pinned target is a stable release.
pub const STABLE: bool = true;
