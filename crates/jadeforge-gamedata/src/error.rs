//! Errors of the game data layer.

use std::path::PathBuf;

/// Result alias for the game data crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong while loading game data.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file could not be read.
    #[error("I/O error on {path}: {source}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// The talk table is damaged or uses an unsupported version.
    #[error("cannot read the talk table: {0}")]
    TalkTable(String),

    /// A 2DA table is damaged.
    #[error("cannot read a 2DA table: {0}")]
    TwoDa(String),

    /// A RIM archive is damaged or uses an unsupported version.
    #[error("cannot read a RIM archive: {0}")]
    Archive(String),

    /// The directory does not look like a Jade Empire installation.
    #[error("{} is not a Jade Empire installation", .0.display())]
    NotAGameInstallation(PathBuf),

    /// The bundled name index is malformed or cannot be generated.
    #[error("cannot read the bundled name index: {0}")]
    Names(String),
}
