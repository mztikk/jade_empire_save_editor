//! Errors of the GFF reader and writer.

/// Result alias for the GFF layer.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong while parsing, editing or writing a GFF block.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The block is structurally damaged.
    #[error("GFF block {block}: {message}")]
    Malformed {
        /// Name of the offending block, e.g. `save.sav`.
        block: String,
        /// Human readable description of the problem.
        message: String,
    },

    /// A value cannot be represented in the file format.
    #[error("corrupt savegame: {0}")]
    Corrupt(String),

    /// A field type is not supported by this build.
    #[error("field type {0} is not supported")]
    UnsupportedType(u32),

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

    /// A label does not fit into the fixed size label table slots.
    #[error("label {0:?} cannot be stored: labels are limited to 16 bytes")]
    LabelTooLong(String),
}
