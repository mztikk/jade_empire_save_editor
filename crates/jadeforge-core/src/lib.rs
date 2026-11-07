//! # `JadeForge` core
//!
//! Reading, editing and writing Jade Empire savegames.
//!
//! The crate is intentionally free of any user interface: the command line
//! tool and the desktop application are thin layers on top of it, and the test
//! suite exercises this crate directly against real savegames.

pub mod backup;
pub mod character;
pub mod error;
pub mod gems;
pub mod henchmen;
pub mod inventory;
pub mod json;
pub mod save;
pub mod screenshot;
pub mod styles;

pub use backup::{BackupEntry, list_backups, restore_backup};
pub use character::{Change, Character, DEFAULT_ALIGNMENT, PlayerLocation, locate_player};
pub use error::{Error, Result};
pub use gems::{AmuletInfo, Gem, GemEffect, GemLocation, sockets_for_tier};
pub use henchmen::Henchman;
pub use inventory::Item;
pub use jadeforge_gff as gff;
pub use jadeforge_gff::{Field, FieldPath, Gff, LocString, Struct, Value};
pub use save::{Block, SaveGame, SaveOptions};
pub use screenshot::SaveImage;
pub use styles::{MAX_STYLE_LEVEL, Style, StyleIdentity};
