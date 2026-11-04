//! JSON export of parsed GFF trees.
//!
//! The export is primarily a debugging and scripting aid: it makes the
//! structure of a savegame inspectable with `jadeforge dump --json` and keeps
//! the format documented in an executable way.

use std::fmt::Write as _;

use serde_json::{Map, Number, Value as Json};

use crate::gff::{Gff, Value};

impl Gff {
    /// Exports the whole block as JSON, starting at the root struct.
    #[must_use]
    pub fn to_json(&self) -> Json {
        self.struct_to_json(self.root, 0)
    }

    /// Exports a single value as JSON.
    #[must_use]
    pub fn value_json(&self, value: &Value) -> Json {
        self.value_to_json(value, 0)
    }

    fn struct_to_json(&self, index: usize, depth: usize) -> Json {
        if depth > 64 {
            return Json::String(format!("<struct {index}: nesting too deep>"));
        }
        let Some(structure) = self.structs.get(index) else {
            return Json::String(format!("<struct {index}: missing>"));
        };
        let mut object = Map::new();
        for field in &structure.fields {
            let name = self
                .label_name(field.label)
                .unwrap_or("<unknown label>")
                .to_owned();
            let name = if object.contains_key(&name) {
                format!("{name}#{}", field.label)
            } else {
                name
            };
            object.insert(name, self.value_to_json(&field.value, depth + 1));
        }
        Json::Object(object)
    }

    fn value_to_json(&self, value: &Value, depth: usize) -> Json {
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
            Value::Struct(index) => self.struct_to_json(*index, depth + 1),
            Value::List(elements) => Json::Array(
                elements
                    .iter()
                    .map(|element| {
                        if depth > 64 {
                            Json::String(format!("<struct {element}>"))
                        } else {
                            self.struct_to_json(*element, depth + 1)
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
