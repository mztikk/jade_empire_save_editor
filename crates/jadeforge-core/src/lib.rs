//! # `JadeForge` core
//!
//! Reading, editing and writing Jade Empire savegames.
//!
//! The crate is intentionally free of any user interface: the command line
//! tool and the desktop application are thin layers on top of it, and the test
//! suite exercises this crate directly against real savegames.

pub mod character;
pub mod error;
pub mod gff;
pub mod henchmen;
pub mod inventory;
pub mod json;
pub mod save;
pub mod screenshot;
pub mod styles;

pub use character::{Change, Character, DEFAULT_ALIGNMENT, PlayerLocation, locate_player};
pub use error::{Error, Result};
pub use gff::{Field, FieldPath, Gff, LocString, Struct, Value};
pub use henchmen::Henchman;
pub use inventory::Item;
pub use save::{Block, SaveGame, SaveOptions};
pub use screenshot::SaveImage;
pub use styles::{MAX_STYLE_LEVEL, Style};
