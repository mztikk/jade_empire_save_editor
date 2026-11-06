//! Scans the production UI and session sources for literal catalog ids.
//!
//! The scan is deliberately textual: it checks the exact call shapes used by
//! the code (`t("id")`, `fill("id"`, `plural(..., "id", "id")`, and the session
//! `tr`/`fill` wrappers). A literal with a space or an uppercase character is
//! treated as an English source string and fails.

#![allow(clippy::expect_used)] // tests fail loudly with a readable diagnostic

use std::path::{Path, PathBuf};

use jadeforge_i18n::has_id;

fn source_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            source_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn decode_rust(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match chars.next() {
            Some('\n') => {
                while chars.peek().is_some_and(|next| next.is_whitespace()) {
                    chars.next();
                }
            }
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// Collects the first `wanted` string literals inside the call starting after
/// the opening parenthesis.
fn call_literals(source: &str, start: usize, wanted: usize) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut literals = Vec::new();
    let mut position = start;
    let mut depth = 1_usize;
    while position < bytes.len() && depth > 0 && literals.len() < wanted {
        match bytes[position] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'"' => {
                let begin = position + 1;
                let mut end = begin;
                let mut escaped = false;
                while end < bytes.len() {
                    if escaped {
                        escaped = false;
                    } else {
                        match bytes[end] {
                            b'\\' => escaped = true,
                            b'"' => break,
                            _ => {}
                        }
                    }
                    end += 1;
                }
                if end < bytes.len() {
                    literals.push(decode_rust(source.get(begin..end).unwrap_or_default()));
                    position = end;
                }
            }
            _ => {}
        }
        position += 1;
    }
    literals
}

fn scan_file(file: &Path, failures: &mut Vec<String>) {
    let Ok(source) = std::fs::read_to_string(file) else {
        return;
    };
    let source = source
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(source.as_str());
    let bytes = source.as_bytes();
    for name in ["t", "fill", "tr", "plural"] {
        let mut index = 0;
        while let Some(found) = source.get(index..).and_then(|rest| rest.find(name)) {
            let at = index + found;
            let after = at + name.len();
            let boundary = at == 0 || {
                let previous = bytes[at - 1];
                !previous.is_ascii_alphanumeric() && previous != b'_'
            };
            let call_start = source
                .get(after..)
                .and_then(|rest| rest.trim_start().starts_with('(').then_some(true));
            if !boundary || call_start != Some(true) {
                index = after;
                continue;
            }
            let mut open = after;
            while open < bytes.len() && bytes[open] != b'(' {
                open += 1;
            }
            let wanted = if name == "plural" { 2 } else { 1 };
            for id in call_literals(source, open + 1, wanted) {
                let valid = !id.is_empty()
                    && id.chars().all(|character| {
                        character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '_'
                            || character == '.'
                    });
                if !valid {
                    failures.push(format!(
                        "{}: {name} receives a non-id literal {id:?} (English source strings must be replaced by ids)",
                        file.display()
                    ));
                } else if !has_id(&id) {
                    failures.push(format!(
                        "{}: {name} id {id:?} is missing from en.toml",
                        file.display()
                    ));
                }
            }
            index = after;
        }
    }
}

#[test]
fn every_literal_ui_id_exists_in_the_english_catalog() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    source_files(&root.join("../jadeforge-gui/src"), &mut files);
    source_files(&root.join("../jadeforge-session/src"), &mut files);
    let mut failures = Vec::new();
    for file in files {
        if file.file_name().is_some_and(|name| name == "i18n.rs") {
            continue;
        }
        scan_file(&file, &mut failures);
    }
    assert!(
        failures.is_empty(),
        "catalog id scan failed:\n{}",
        failures.join("\n")
    );
}
