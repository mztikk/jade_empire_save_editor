//! JSON export of parsed GFF trees.
//!
//! The export is primarily a debugging and scripting aid: it makes the
//! structure of a savegame inspectable with `jadeforge dump --json` and keeps
//! the format documented in an executable way.

use std::collections::HashSet;
use std::fmt::Write as _;

use jadeforge_gff::{Gff, Value};
use serde_json::{Map, Number, Value as Json};

/// Maximum number of struct nodes a single export may emit.
const NODE_BUDGET: usize = 100_000;

/// Exports the whole block as JSON, starting at the root struct.
#[must_use]
pub fn to_json(gff: &Gff) -> Json {
    let mut budget = NODE_BUDGET;
    struct_to_json(gff, gff.root, 0, &mut HashSet::new(), &mut budget)
}

/// Exports a single value as JSON.
#[must_use]
pub fn value_json(gff: &Gff, value: &Value) -> Json {
    let mut budget = NODE_BUDGET;
    value_to_json(gff, value, 0, &mut HashSet::new(), &mut budget)
}

fn struct_to_json(
    gff: &Gff,
    index: usize,
    depth: usize,
    visited: &mut HashSet<usize>,
    budget: &mut usize,
) -> Json {
    if depth > 64 {
        return Json::String(format!("<struct {index}: nesting too deep>"));
    }
    if *budget == 0 {
        return Json::String(format!("<struct {index}: export budget exhausted>"));
    }
    *budget -= 1;
    let Some(structure) = gff.structs.get(index) else {
        return Json::String(format!("<struct {index}: missing>"));
    };
    if !visited.insert(index) {
        return Json::String(format!("<struct {index}: cycle>"));
    }
    let mut object = Map::new();
    for field in &structure.fields {
        let name = gff
            .label_name(field.label)
            .unwrap_or("<unknown label>")
            .to_owned();
        let mut key = name.clone();
        let mut suffix = 0_usize;
        while object.contains_key(&key) {
            suffix += 1;
            key = format!("{name}#{suffix}");
        }
        object.insert(
            key,
            value_to_json(gff, &field.value, depth + 1, visited, budget),
        );
    }
    visited.remove(&index);
    Json::Object(object)
}

fn value_to_json(
    gff: &Gff,
    value: &Value,
    depth: usize,
    visited: &mut HashSet<usize>,
    budget: &mut usize,
) -> Json {
    match value {
        Value::Byte(value) => Json::from(*value),
        Value::Char(value) => Json::from(*value),
        Value::Word(value) => Json::from(*value),
        Value::Short(value) => Json::from(*value),
        Value::DWord(value) => Json::from(*value),
        Value::Int(value) => Json::from(*value),
        Value::DWord64(value) => Json::from(*value),
        Value::Int64(value) => Json::from(*value),
        Value::Float(value) => float_json(f64::from(*value)),
        Value::Double(value) => float_json(*value),
        Value::String(value) | Value::ResRef(value) => Json::String(value.clone()),
        Value::LocString(locstring) => {
            let mut object = Map::new();
            object.insert(
                "strref".to_owned(),
                locstring.strref.map_or(Json::Null, Json::from),
            );
            object.insert(
                "text".to_owned(),
                locstring.text.clone().map_or(Json::Null, Json::String),
            );
            Json::Object(object)
        }
        Value::Void(bytes) => Json::String(hex(bytes)),
        Value::Struct(index) => struct_to_json(gff, *index, depth + 1, visited, budget),
        Value::List(elements) => Json::Array(
            elements
                .iter()
                .map(|element| {
                    if depth > 64 {
                        Json::String(format!("<struct {element}>"))
                    } else {
                        struct_to_json(gff, *element, depth + 1, visited, budget)
                    }
                })
                .collect(),
        ),
        Value::Quaternion(values) => Json::Array(
            values
                .iter()
                .map(|value| float_json(f64::from(*value)))
                .collect(),
        ),
        Value::Vector3(values) => Json::Array(
            values
                .iter()
                .map(|value| float_json(f64::from(*value)))
                .collect(),
        ),
    }
}

fn float_json(value: f64) -> Json {
    Number::from_f64(value).map_or_else(|| Json::String(value.to_string()), Json::Number)
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing into a String cannot fail.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use jadeforge_gff::{Field, Struct, VERSION};

    #[test]
    fn a_cyclic_struct_graph_is_marked_instead_of_expanding_forever() {
        let gff = Gff {
            version: VERSION,
            labels: vec!["Next".to_owned()],
            structs: vec![
                Struct {
                    type_id: u32::MAX,
                    fields: vec![Field {
                        label: 0,
                        type_id: 14,
                        value: Value::Struct(1),
                    }],
                },
                Struct {
                    type_id: 0,
                    fields: vec![Field {
                        label: 0,
                        type_id: 14,
                        value: Value::Struct(0),
                    }],
                },
            ],
            root: 0,
        };
        let text = serde_json::to_string(&to_json(&gff)).expect("json");
        assert!(text.contains("cycle"), "{text}");
        assert!(text.len() < 1024, "the cycle marker must stay small");
    }
}
