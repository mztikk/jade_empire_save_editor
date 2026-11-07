//! Reader and writer for the GFF variant used by Jade Empire savegames.
//!
//! A savegame is a container of named blocks; every block is a self describing
//! tree of structs and fields. The layout of a block is:
//!
//! ```text
//! 0x00  "SAV "                      magic
//! 0x04  "V3.2"                      version
//! 0x08  u32 structOffset            always 56
//! 0x0c  u32 structCount
//! 0x10  u32 fieldOffset
//! 0x14  u32 fieldCount
//! 0x18  u32 labelOffset
//! 0x1c  u32 labelCount
//! 0x20  u32 fieldDataOffset
//! 0x24  u32 fieldDataSize
//! 0x28  u32 fieldIndicesOffset
//! 0x2c  u32 fieldIndicesSize
//! 0x30  u32 listIndicesOffset
//! 0x34  u32 listIndicesSize
//! 0x38  struct table   (structCount   * 12 bytes)
//!       field table    (fieldCount    * 12 bytes)
//!       label table    (labelCount    * 16 bytes)
//!       field data     (fieldDataSize bytes)
//!       field indices  (fieldIndicesSize bytes)
//!       list indices   (listIndicesSize  bytes)
//! ```
//!
//! See `docs/FORMAT.md` for the full description, including the surprising
//! `fieldCount == 1` encoding of structs.

mod path;
mod tables;
mod value;
mod write;

pub mod error;

use std::collections::HashSet;

pub use error::{Error, Result};
pub use path::FieldPath;
pub use value::{LocString, Value};

/// Four byte magic that starts every block.
pub const MAGIC: [u8; 4] = *b"SAV ";
/// Version string of the block format.
pub const VERSION: [u8; 4] = *b"V3.2";
/// Size of the block header in bytes.
pub const HEADER_SIZE: usize = 56;
/// Size of a struct table entry in bytes.
pub const STRUCT_ENTRY_SIZE: usize = 12;
/// Size of a field table entry in bytes.
pub const FIELD_ENTRY_SIZE: usize = 12;
/// Size of a label table entry (fixed size, NUL padded) in bytes.
pub const LABEL_SIZE: usize = 16;
/// Maximum number of bytes a label may occupy. Labels that use all 16 bytes
/// are not NUL terminated.
pub const LABEL_MAX_LEN: usize = LABEL_SIZE;
/// Field type tag of a sub-struct reference.
pub const TYPE_STRUCT: u32 = 14;
/// Field type tag of a list of sub-structs.
pub const TYPE_LIST: u32 = 15;

/// A struct inside a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
    /// Struct type id. The root struct uses `0xffff_ffff`.
    pub type_id: u32,
    /// Fields in the order they are stored.
    pub fields: Vec<Field>,
}

/// A single field of a [`Struct`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Index into [`Gff::labels`].
    pub label: u32,
    /// Type tag as stored on disk. Kept verbatim so that a round trip never
    /// changes types that we only partially understand.
    pub type_id: u32,
    /// The decoded value.
    pub value: Value,
}

/// A parsed GFF block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gff {
    /// Version tag, normally `V3.2`.
    pub version: [u8; 4],
    /// Label table; every field references an entry of this table.
    pub labels: Vec<String>,
    /// All structs, including any that are not reachable from the root.
    pub structs: Vec<Struct>,
    /// Index of the root struct; entry zero for every known savegame.
    pub root: usize,
}

impl Gff {
    /// Parses a GFF block.
    ///
    /// `block` is only used to produce readable error messages.
    ///
    /// Parsing is strict: the tiling of the table sections, every field and
    /// struct reference, duplicate references and the total amount of decoded
    /// data are checked. A block that does not match the format produces an
    /// error instead of a partially decoded tree.
    pub fn parse(block: &str, bytes: &[u8]) -> Result<Self> {
        let tables = tables::Tables::parse(block, bytes)?;
        let field_data = tables.field_data(bytes);
        let list_indices = tables.list_indices(bytes);
        let mut budget = Budget::new(bytes.len());

        let mut values = Vec::with_capacity(tables.fields.len());
        for entry in &tables.fields {
            let value = decode_value(block, entry, field_data, list_indices)?;
            budget.consume(block, &value)?;
            values.push(value);
        }

        let mut structs = Vec::with_capacity(tables.structs.len());
        for entry in &tables.structs {
            let mut fields = Vec::with_capacity(entry.fields.len());
            for &index in &entry.fields {
                let field = &tables.fields[index];
                let value = values[index].clone();
                budget.consume(block, &value)?;
                fields.push(Field {
                    label: field.label,
                    type_id: field.type_id,
                    value,
                });
            }
            structs.push(Struct {
                type_id: entry.type_id,
                fields,
            });
        }

        Ok(Self {
            version: tables.version,
            labels: tables.labels,
            structs,
            root: tables.root,
        })
    }

    /// Decodes only the fields named by `paths`, without materialising the
    /// rest of the tree.
    ///
    /// This is used for the savegame list, which needs a handful of values from
    /// a block that may contain tens of thousands of fields. The header and
    /// tables are validated exactly like [`Gff::parse`], but values for fields
    /// that are not requested are never decoded. A missing path, a wrong
    /// intermediate type, or an indexed leaf yields `None`. Structural table or
    /// list corruption still produces an error.
    ///
    /// The grammar is [`FieldPath`]'s dotted path grammar. A path whose final
    /// segment has a list index is not a field, so it yields `None`, matching
    /// [`Gff::get`].
    pub fn selected_fields(
        block: &str,
        bytes: &[u8],
        paths: &[&str],
    ) -> Result<Vec<Option<Value>>> {
        let tables = tables::Tables::parse(block, bytes)?;
        let field_data = tables.field_data(bytes);
        let list_indices = tables.list_indices(bytes);
        let mut budget = Budget::new(bytes.len());
        let mut selected = Vec::with_capacity(paths.len());

        for path in paths {
            let Ok(path) = FieldPath::parse(path) else {
                selected.push(None);
                continue;
            };
            let segments = path.segments();
            if segments.is_empty() {
                selected.push(None);
                continue;
            }
            let mut current = tables.root;
            let mut value = None;
            let mut missing = false;
            for (position, segment) in segments.iter().enumerate() {
                let Some(structure) = tables.structs.get(current) else {
                    missing = true;
                    break;
                };
                let Some(field_index) = structure.fields.iter().copied().find(|index| {
                    let field = &tables.fields[*index];
                    tables.labels[field.label as usize] == segment.name
                }) else {
                    missing = true;
                    break;
                };
                let field = tables.fields[field_index];
                if position + 1 == segments.len() {
                    if segment.index.is_some() {
                        missing = true;
                        break;
                    }
                    let decoded = decode_value(block, &field, field_data, list_indices)?;
                    budget.consume(block, &decoded)?;
                    value = Some(decoded);
                } else {
                    let target = match (field.type_id, segment.index) {
                        (TYPE_STRUCT, None) => field.data as usize,
                        (TYPE_LIST, Some(index)) => {
                            let Ok(elements) = tables::read_list(list_indices, field.data) else {
                                missing = true;
                                break;
                            };
                            let Some(target) = elements.get(index).copied() else {
                                missing = true;
                                break;
                            };
                            target
                        }
                        _ => {
                            missing = true;
                            break;
                        }
                    };
                    if target >= tables.structs.len() {
                        missing = true;
                        break;
                    }
                    current = target;
                }
            }
            selected.push(if missing { None } else { value });
        }
        Ok(selected)
    }

    /// Returns the name of a label, or `None` when the index is out of range.
    #[must_use]
    pub fn label_name(&self, index: u32) -> Option<&str> {
        self.labels.get(index as usize).map(String::as_str)
    }

    /// Returns the index of a label with the given name.
    #[must_use]
    pub fn label_index(&self, name: &str) -> Option<u32> {
        self.labels
            .iter()
            .position(|label| label == name)
            .map(|index| u32::try_from(index).ok())?
    }

    /// Returns the label index for `name`, adding it to the label table when it
    /// does not exist yet.
    pub fn intern_label(&mut self, name: &str) -> Result<u32> {
        if let Some(index) = self.label_index(name) {
            return Ok(index);
        }
        if name.len() > LABEL_MAX_LEN {
            return Err(Error::LabelTooLong(name.to_owned()));
        }
        let index = u32::try_from(self.labels.len())
            .map_err(|_| Error::Corrupt("the label table is too large".to_owned()))?;
        self.labels.push(name.to_owned());
        Ok(index)
    }

    /// Returns the root struct, if the tree has one.
    #[must_use]
    pub fn root(&self) -> Option<&Struct> {
        self.structs.get(self.root)
    }

    /// Returns a struct by index.
    #[must_use]
    pub fn struct_at(&self, index: usize) -> Option<&Struct> {
        self.structs.get(index)
    }

    /// Returns a mutable struct by index.
    #[must_use]
    pub fn struct_at_mut(&mut self, index: usize) -> Option<&mut Struct> {
        self.structs.get_mut(index)
    }

    /// Finds a field by label inside a struct.
    #[must_use]
    pub fn find(&self, struct_index: usize, label: &str) -> Option<&Field> {
        let structure = self.structs.get(struct_index)?;
        structure
            .fields
            .iter()
            .find(|field| self.label_name(field.label) == Some(label))
    }

    /// Finds a field by label inside a struct, mutably.
    #[must_use]
    pub fn find_mut(&mut self, struct_index: usize, label: &str) -> Option<&mut Field> {
        let index = self.label_index(label)?;
        self.structs
            .get_mut(struct_index)?
            .fields
            .iter_mut()
            .find(|field| field.label == index)
    }

    /// Resolves a path that points at a struct.
    pub fn resolve_struct(&self, path: &FieldPath) -> Option<usize> {
        let mut current = self.root;
        for segment in path.segments() {
            let structure = self.structs.get(current)?;
            let position = structure
                .fields
                .iter()
                .position(|field| self.label_name(field.label) == Some(segment.name.as_str()))?;
            current = match (&structure.fields[position].value, segment.index) {
                (Value::Struct(target), None) => *target,
                (Value::List(elements), Some(index)) => *elements.get(index)?,
                _ => return None,
            };
        }
        Some(current)
    }

    /// Adds a field described by a dotted path.
    ///
    /// The parent struct has to exist; the label is interned when it is not
    /// known yet.
    pub fn add(&mut self, path: impl AsRef<str>, value: Value) -> Result<()> {
        let path = FieldPath::parse(path.as_ref())?;
        let segments = path.segments();
        let (leaf, _) = segments.split_last().ok_or_else(|| Error::FieldNotFound {
            path: path.to_string(),
        })?;
        if leaf.index.is_some() {
            return Err(Error::FieldNotFound {
                path: path.to_string(),
            });
        }
        let parent = match path.parent() {
            Some(parent) => self
                .resolve_struct(&parent)
                .ok_or_else(|| Error::FieldNotFound {
                    path: parent.to_string(),
                })?,
            None => self.root,
        };
        self.add_field(parent, &leaf.name, value)?;
        Ok(())
    }

    /// Removes a field described by a dotted path.
    ///
    /// An indexed leaf is rejected: the path names a list element, not a field,
    /// so deleting the whole list field would be surprising and destructive.
    pub fn remove(&mut self, path: impl AsRef<str>) -> Result<Field> {
        let path = FieldPath::parse(path.as_ref())?;
        let segments = path.segments();
        let (leaf, _) = segments.split_last().ok_or_else(|| Error::FieldNotFound {
            path: path.to_string(),
        })?;
        if leaf.index.is_some() {
            return Err(Error::FieldNotFound {
                path: path.to_string(),
            });
        }
        let parent = match path.parent() {
            Some(parent) => self
                .resolve_struct(&parent)
                .ok_or_else(|| Error::FieldNotFound {
                    path: parent.to_string(),
                })?,
            None => self.root,
        };
        self.remove_field(parent, &leaf.name)
    }

    /// Resolves a dotted path such as `Stats.Focus.STAT_CUR` or
    /// `Henchmen.HenchList[2].Level` to the struct that contains the field and
    /// the position of the field inside that struct.
    pub fn resolve(&self, path: &FieldPath) -> Option<(usize, usize)> {
        let mut current = self.root;
        for (position, segment) in path.segments().iter().enumerate() {
            let struct_index = self.structs.get(current)?;
            let field_index = struct_index
                .fields
                .iter()
                .position(|field| self.label_name(field.label) == Some(segment.name.as_str()))?;
            if position + 1 == path.segments().len() {
                if segment.index.is_some() {
                    return None;
                }
                return Some((current, field_index));
            }
            let field = &struct_index.fields[field_index];
            current = match (&field.value, segment.index) {
                (Value::Struct(target), None) => *target,
                (Value::List(elements), Some(index)) => *elements.get(index)?,
                _ => return None,
            };
        }
        None
    }

    /// Looks up a field by path.
    pub fn get(&self, path: impl AsRef<str>) -> Result<&Field> {
        let path = FieldPath::parse(path.as_ref())?;
        let (struct_index, field_index) =
            self.resolve(&path).ok_or_else(|| Error::FieldNotFound {
                path: path.to_string(),
            })?;
        let structure = self
            .structs
            .get(struct_index)
            .ok_or_else(|| Error::FieldNotFound {
                path: path.to_string(),
            })?;
        structure
            .fields
            .get(field_index)
            .ok_or_else(|| Error::FieldNotFound {
                path: path.to_string(),
            })
    }

    /// Looks up a field by path, mutably.
    pub fn get_mut(&mut self, path: impl AsRef<str>) -> Result<&mut Field> {
        let path = FieldPath::parse(path.as_ref())?;
        let (struct_index, field_index) =
            self.resolve(&path).ok_or_else(|| Error::FieldNotFound {
                path: path.to_string(),
            })?;
        self.structs
            .get_mut(struct_index)
            .and_then(|structure| structure.fields.get_mut(field_index))
            .ok_or_else(|| Error::FieldNotFound {
                path: path.to_string(),
            })
    }

    /// Replaces the value of a field. The new value has to have the same kind
    /// as the old one, which keeps the on-disk type tag stable.
    pub fn set(&mut self, path: impl AsRef<str>, value: Value) -> Result<()> {
        let path = path.as_ref();
        let field = self.get_mut(path)?;
        if !is_compatible(field.type_id, &value) {
            return Err(Error::TypeMismatch {
                path: path.to_owned(),
                expected: field.value.type_name(),
                actual: value.type_name(),
            });
        }
        field.value = value;
        Ok(())
    }

    /// Adds a field to a struct. The label is interned when it is unknown.
    pub fn add_field(
        &mut self,
        struct_index: usize,
        label: &str,
        value: Value,
    ) -> Result<&mut Field> {
        let type_id = value.type_id();
        self.add_field_typed(struct_index, label, type_id, value)
    }

    /// Adds a field with an explicit on-disk type tag.
    ///
    /// This is used by typed editors that clone a field layout and must keep
    /// the original tag (for example, an `int` reference must not silently
    /// become a `dword`). The type tag and value are checked for compatibility.
    pub fn add_field_typed(
        &mut self,
        struct_index: usize,
        label: &str,
        type_id: u32,
        value: Value,
    ) -> Result<&mut Field> {
        if !is_compatible(type_id, &value) {
            return Err(Error::TypeMismatch {
                path: label.to_owned(),
                expected: type_name_for_id(type_id),
                actual: value.type_name(),
            });
        }
        let structure = self
            .structs
            .get(struct_index)
            .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?;
        if label.len() > LABEL_MAX_LEN {
            return Err(Error::LabelTooLong(label.to_owned()));
        }
        if let Some(index) = self.label_index(label)
            && structure.fields.iter().any(|field| field.label == index)
        {
            return Err(Error::Corrupt(format!(
                "struct {struct_index} already has a field named {label:?}"
            )));
        }
        let label = self.intern_label(label)?;
        let structure = self
            .structs
            .get_mut(struct_index)
            .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?;
        structure.fields.push(Field {
            label,
            type_id,
            value,
        });
        structure
            .fields
            .last_mut()
            .ok_or_else(|| Error::Corrupt("the field that was just pushed vanished".to_owned()))
    }

    /// Removes a field from a struct.
    pub fn remove_field(&mut self, struct_index: usize, label: &str) -> Result<Field> {
        let index = self
            .label_index(label)
            .ok_or_else(|| Error::FieldNotFound {
                path: label.to_owned(),
            })?;
        let structure = self
            .structs
            .get_mut(struct_index)
            .ok_or_else(|| Error::FieldNotFound {
                path: label.to_owned(),
            })?;
        let position = structure
            .fields
            .iter()
            .position(|field| field.label == index)
            .ok_or_else(|| Error::FieldNotFound {
                path: label.to_owned(),
            })?;
        Ok(structure.fields.remove(position))
    }

    /// Appends a new struct and returns its index.
    pub fn push_struct(&mut self, type_id: u32) -> usize {
        self.structs.push(Struct {
            type_id,
            fields: Vec::new(),
        });
        self.structs.len() - 1
    }

    /// Total number of fields across all structs.
    #[must_use]
    pub fn field_count(&self) -> usize {
        self.structs
            .iter()
            .map(|structure| structure.fields.len())
            .sum()
    }

    /// Checks the invariants that the on-disk format requires.
    pub fn validate(&self, block: &str) -> Result<()> {
        let err = |message: String| Error::Malformed {
            block: block.to_owned(),
            message,
        };
        if &self.version[..2] != b"V3" {
            return Err(err(format!(
                "unsupported version {:?}",
                String::from_utf8_lossy(&self.version)
            )));
        }
        if self.structs.is_empty() {
            return Err(err("block has no structs".to_owned()));
        }
        if self.root >= self.structs.len() {
            return Err(err(format!(
                "root struct {} is out of bounds ({} structs)",
                self.root,
                self.structs.len()
            )));
        }
        if self.structs[self.root].type_id != u32::MAX {
            return Err(err(format!(
                "root struct {} has type {}, expected 0xffff_ffff",
                self.root, self.structs[self.root].type_id
            )));
        }
        let roots: Vec<usize> = self
            .structs
            .iter()
            .enumerate()
            .filter(|(_, structure)| structure.type_id == u32::MAX)
            .map(|(index, _)| index)
            .collect();
        if roots != [self.root] {
            return Err(err(format!(
                "the root struct must be the only struct with type 0xffff_ffff, found {roots:?} with root {}",
                self.root
            )));
        }
        let mut seen_labels = HashSet::with_capacity(self.labels.len());
        for (index, label) in self.labels.iter().enumerate() {
            if label.len() > LABEL_MAX_LEN || label.as_bytes().contains(&0) {
                return Err(err(format!(
                    "label {index} ({label:?}) does not fit into {LABEL_MAX_LEN} bytes"
                )));
            }
            if !seen_labels.insert(label.as_str()) {
                return Err(err(format!("label table contains {label:?} twice")));
            }
        }
        for (index, structure) in self.structs.iter().enumerate() {
            let mut seen = HashSet::new();
            for field in &structure.fields {
                let Some(label) = self.labels.get(field.label as usize) else {
                    return Err(err(format!(
                        "struct {index} uses label {} which does not exist",
                        field.label
                    )));
                };
                if !seen.insert(field.label) {
                    return Err(err(format!(
                        "struct {index} contains the label {label:?} twice"
                    )));
                }
                if !is_compatible(field.type_id, &field.value) {
                    return Err(err(format!(
                        "struct {index} field {label:?} has type tag {} but holds {}",
                        field.type_id,
                        field.value.type_name()
                    )));
                }
                if let Value::ResRef(value) = &field.value
                    && value.len() > 255
                {
                    return Err(err(format!(
                        "struct {index} field {label:?} has a {}-byte ResRef; the format allows at most 255",
                        value.len()
                    )));
                }
                match &field.value {
                    Value::Struct(target) if *target >= self.structs.len() => {
                        return Err(err(format!(
                            "struct {index} field {label:?} points at struct {target}, out of bounds"
                        )));
                    }
                    Value::List(elements) => {
                        for element in elements {
                            if *element >= self.structs.len() {
                                return Err(err(format!(
                                    "struct {index} field {label:?} contains a list with out of bounds struct {element}"
                                )));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// True when this struct is reachable from the root.
    #[must_use]
    pub fn reachable(&self) -> Vec<bool> {
        let mut seen = vec![false; self.structs.len()];
        let mut stack = vec![self.root];
        while let Some(index) = stack.pop() {
            if index >= seen.len() || seen[index] {
                continue;
            }
            seen[index] = true;
            for field in &self.structs[index].fields {
                match &field.value {
                    Value::Struct(target) => stack.push(*target),
                    Value::List(elements) => stack.extend(elements.iter().copied()),
                    _ => {}
                }
            }
        }
        seen
    }
}

/// True when a field's on-disk type tag can hold the given value.
fn is_compatible(type_id: u32, value: &Value) -> bool {
    type_id == value.type_id()
        || (matches!(type_id, 12 | 18) && matches!(value, Value::LocString(_)))
}

/// Human readable name of an on-disk field type tag.
fn type_name_for_id(type_id: u32) -> &'static str {
    match type_id {
        0 => "byte",
        1 => "char",
        2 => "word",
        3 => "short",
        4 => "dword",
        5 => "int",
        6 => "dword64",
        7 => "int64",
        8 => "float",
        9 => "double",
        10 => "string",
        11 => "resref",
        12 | 18 => "locstring",
        13 => "void",
        14 => "struct",
        15 => "list",
        16 => "quaternion",
        17 => "vector3",
        _ => "unknown type",
    }
}

/// True for field types whose payload lives in the field-data section.
fn is_out_of_line(type_id: u32) -> bool {
    matches!(type_id, 6 | 7 | 9 | 10 | 11 | 12 | 13 | 16 | 17 | 18)
}

/// Decodes one field table entry.
fn decode_value(
    block: &str,
    field: &tables::FieldEntry,
    field_data: &[u8],
    list_indices: &[u8],
) -> Result<Value> {
    let err = |message: String| Error::Malformed {
        block: block.to_owned(),
        message,
    };
    if field.type_id == TYPE_LIST {
        return tables::read_list(list_indices, field.data)
            .map(Value::List)
            .map_err(|_| err(format!("list at {} is malformed", field.data)));
    }
    if is_out_of_line(field.type_id) {
        Value::decode_field_data(field.type_id, field.data, field_data, block)
    } else {
        Value::decode_inline(field.type_id, field.data)
            .map_err(|_| err(format!("field references unknown type {}", field.type_id)))
    }
}

/// Approximate number of bytes a decoded value owns.
fn value_weight(value: &Value) -> usize {
    match value {
        Value::String(value) | Value::ResRef(value) => value.len().saturating_add(32),
        Value::LocString(locstring) => locstring
            .text
            .as_ref()
            .map_or(64, |text| text.len().saturating_add(32)),
        Value::Void(bytes) => bytes.len().saturating_add(32),
        Value::List(elements) => elements.len().saturating_mul(8).saturating_add(16),
        Value::Quaternion(_) | Value::Vector3(_) => 32,
        _ => 16,
    }
}

/// Guards against a small block decoding into a much larger tree.
struct Budget {
    used: usize,
    limit: usize,
}

/// Baseline decoded-data allowance for small blocks.
///
/// The writer de-duplicates field payloads, but the parser still materializes
/// one value per field entry and one clone per struct reference. A small block
/// can therefore represent several megabytes of decoded values without being
/// hostile. The per-reference charge still rejects large amplification.
const MIN_DECODED_BUDGET: usize = 16 * 1024 * 1024;

impl Budget {
    fn new(block_len: usize) -> Self {
        // 4x plus one MiB leaves a wide margin for real saves; the baseline
        // keeps valid, compactly de-duplicated writer output parseable.
        Self {
            used: 0,
            limit: block_len
                .saturating_mul(4)
                .saturating_add(1 << 20)
                .max(MIN_DECODED_BUDGET),
        }
    }

    fn consume(&mut self, block: &str, value: &Value) -> Result<()> {
        let amount = value_weight(value);
        self.used = self
            .used
            .checked_add(amount)
            .ok_or_else(|| Error::Malformed {
                block: block.to_owned(),
                message: "decoded data size overflows the address space".to_owned(),
            })?;
        if self.used > self.limit {
            return Err(Error::Malformed {
                block: block.to_owned(),
                message: format!(
                    "decoded data exceeds the {} byte budget for this block",
                    self.limit
                ),
            });
        }
        Ok(())
    }
}
