//! Error type shared by every part of `JadeForge`.

use std::path::PathBuf;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong while reading, editing or writing a savegame.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file system operation failed.
    #[error("I/O error on {path}: {source}")]
    Io {
        /// Path the operation was performed on.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// The file does not start with a Jade Empire savegame header.
    #[error("not a Jade Empire savegame: {0}")]
    NotASavegame(String),

    /// The file is structurally damaged.
    #[error("corrupt savegame: {0}")]
    Corrupt(String),

    /// A single GFF block is structurally damaged.
    #[error("GFF block {block}: {message}")]
    Gff {
        /// Name of the offending block, e.g. `save.sav`.
        block: String,
        /// Human readable description of the problem.
        message: String,
    },

    /// The given field path does not exist.
    #[error("field {path:?} does not exist")]
    FieldNotFound {
        /// Path that was looked up.
        path: String,
    },

    /// A field exists but does not hold the expected kind of value.
    #[error("field {path:?} holds {actual}, expected {expected}")]
    TypeMismatch {
        /// Path of the field.
        path: String,
        /// What the caller asked for.
        expected: &'static str,
        /// What the field actually contains.
        actual: &'static str,
    },

    /// A value is outside of the range the savegame format can represent.
    #[error("value {value} is out of range for {path} ({min}..={max})")]
    OutOfRange {
        /// Path of the field.
        path: String,
        /// The rejected value.
        value: String,
        /// Smallest allowed value.
        min: String,
        /// Largest allowed value.
        max: String,
    },

    /// A label does not fit into the fixed size label table slots.
    #[error("label {0:?} cannot be stored: labels are limited to 16 bytes")]
    LabelTooLong(String),

    /// The newly written savegame did not survive a round trip.
    #[error("savegame verification failed: {0}")]
    Verification(String),

    /// A field type is not supported by this build.
    #[error("field type {0} is not supported")]
    UnsupportedType(u32),

    /// A requested list element does not exist.
    #[error("list {path:?} has no element at index {index} (length {len})")]
    IndexOutOfBounds {
        /// Path of the list.
        path: String,
        /// Requested index.
        index: usize,
        /// Current length.
        len: usize,
    },
}

impl Error {
    /// Helper to build an [`Error::Io`] with the path attached.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
