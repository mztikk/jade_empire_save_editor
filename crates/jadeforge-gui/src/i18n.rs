//! Interface catalog re-exports.
//!
//! The catalogs and the locale selection live in `jadeforge-i18n`; the views
//! keep using `crate::i18n::*` so the call sites stay short.

pub use jadeforge_i18n::{
    Key, Locale, current, fill, has_id, install, is_complete, join_list, keys, locale_label,
    plural, t,
};
