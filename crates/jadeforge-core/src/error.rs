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

    /// A single GFF block is damaged or cannot be written.
    #[error(transparent)]
    Gff(jadeforge_gff::Error),

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

    /// The newly written savegame did not survive a round trip.
    #[error("savegame verification failed: {0}")]
    Verification(String),

    /// The file changed on disk after it was opened.
    #[error("savegame {path} changed on disk since it was opened; reload before saving")]
    Conflict {
        /// Path that changed.
        path: PathBuf,
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

impl From<jadeforge_gff::Error> for Error {
    fn from(error: jadeforge_gff::Error) -> Self {
        match error {
            jadeforge_gff::Error::FieldNotFound { path } => Self::FieldNotFound { path },
            jadeforge_gff::Error::TypeMismatch {
                path,
                expected,
                actual,
            } => Self::TypeMismatch {
                path,
                expected,
                actual,
            },
            other => Self::Gff(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gff_errors_keep_their_context_when_converted() {
        let error: Error = jadeforge_gff::Error::FieldNotFound {
            path: "Stats.Gold".to_owned(),
        }
        .into();
        assert!(matches!(error, Error::FieldNotFound { .. }));
        assert!(error.to_string().contains("Stats.Gold"));

        let error: Error = jadeforge_gff::Error::TypeMismatch {
            path: "Stats.Gold".to_owned(),
            expected: "dword",
            actual: "word",
        }
        .into();
        assert!(matches!(error, Error::TypeMismatch { .. }));
        assert!(error.to_string().contains("dword"));

        let error: Error = jadeforge_gff::Error::Malformed {
            block: "save.sav".to_owned(),
            message: "bad magic".to_owned(),
        }
        .into();
        assert!(matches!(error, Error::Gff(_)));
        assert!(error.to_string().contains("save.sav"));
    }
}
