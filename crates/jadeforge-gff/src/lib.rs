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
mod value;
mod write;

pub mod error;

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
    #[allow(clippy::too_many_lines)] // the parser mirrors the on-disk layout
    pub fn parse(block: &str, bytes: &[u8]) -> Result<Self> {
        let err = |message: String| Error::Malformed {
            block: block.to_owned(),
            message,
        };

        if bytes.len() < HEADER_SIZE {
            return Err(err(format!(
                "block is {} bytes, smaller than the {HEADER_SIZE} byte header",
                bytes.len()
            )));
        }
        if bytes[..4] != MAGIC {
            return Err(err(format!(
                "bad magic {:?}, expected {:?}",
                &bytes[..4],
                MAGIC
            )));
        }
        let mut version = [0_u8; 4];
        version.copy_from_slice(&bytes[4..8]);
        if &version[..2] != b"V3" {
            return Err(err(format!(
                "unsupported version {:?}",
                String::from_utf8_lossy(&version)
            )));
        }

        let word = |index: usize| -> Result<u32> {
            let offset = 8 + index * 4;
            bytes
                .get(offset..offset + 4)
                .and_then(|slice| slice.try_into().ok())
                .map(u32::from_le_bytes)
                .ok_or_else(|| err(format!("header word {index} is truncated")))
        };
        let struct_offset = word(0)? as usize;
        let struct_count = word(1)? as usize;
        let field_offset = word(2)? as usize;
        let field_count = word(3)? as usize;
        let label_offset = word(4)? as usize;
        let label_count = word(5)? as usize;
        let field_data_offset = word(6)? as usize;
        let field_data_size = word(7)? as usize;
        let field_indices_offset = word(8)? as usize;
        let field_indices_size = word(9)? as usize;
        let list_indices_offset = word(10)? as usize;
        let list_indices_size = word(11)? as usize;

        let section_end = |offset: usize, size: usize, what: &str| -> Result<usize> {
            let end = offset
                .checked_add(size)
                .ok_or_else(|| err(format!("{what} overflows the address space")))?;
            if end > bytes.len() {
                return Err(err(format!(
                    "{what} ends at {end}, beyond the block size {}",
                    bytes.len()
                )));
            }
            Ok(end)
        };
        section_end(
            struct_offset,
            struct_count * STRUCT_ENTRY_SIZE,
            "struct table",
        )?;
        section_end(field_offset, field_count * FIELD_ENTRY_SIZE, "field table")?;
        section_end(label_offset, label_count * LABEL_SIZE, "label table")?;
        section_end(field_data_offset, field_data_size, "field data")?;
        section_end(field_indices_offset, field_indices_size, "field indices")?;
        section_end(list_indices_offset, list_indices_size, "list indices")?;

        let mut labels = Vec::with_capacity(label_count);
        for index in 0..label_count {
            let start = label_offset + index * LABEL_SIZE;
            let slot = &bytes[start..start + LABEL_SIZE];
            let end = slot
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(LABEL_SIZE);
            let name = std::str::from_utf8(&slot[..end])
                .map_err(|_| err(format!("label {index} is not valid UTF-8")))?
                .to_owned();
            labels.push(name);
        }

        let block_size = list_indices_offset
            .checked_add(list_indices_size)
            .ok_or_else(|| err("section offsets overflow".to_owned()))?;
        if block_size != bytes.len() {
            return Err(err(format!(
                "block size is {} bytes but the sections describe {block_size} bytes",
                bytes.len()
            )));
        }

        let field_data = &bytes[field_data_offset..field_data_offset + field_data_size];
        let field_indices = &bytes[field_indices_offset..field_indices_offset + field_indices_size];
        let list_indices = &bytes[list_indices_offset..list_indices_offset + list_indices_size];

        let read_u32 = |buffer: &[u8], offset: usize, what: &str| -> Result<u32> {
            buffer
                .get(offset..offset + 4)
                .and_then(|slice| slice.try_into().ok())
                .map(u32::from_le_bytes)
                .ok_or_else(|| err(format!("{what} at offset {offset} is out of bounds")))
        };

        let mut fields = Vec::with_capacity(field_count);
        for index in 0..field_count {
            let start = field_offset + index * FIELD_ENTRY_SIZE;
            let type_id = read_u32(bytes, start, "field type")?;
            let label = read_u32(bytes, start + 4, "field label")?;
            let data = read_u32(bytes, start + 8, "field data")?;
            if label as usize >= label_count {
                return Err(err(format!(
                    "field {index} references label {label}, but only {label_count} labels exist"
                )));
            }
            let value = if type_id == TYPE_LIST {
                let count = read_u32(list_indices, data as usize, "list count")? as usize;
                let mut elements = Vec::with_capacity(count.min(1 << 16));
                for element in 0..count {
                    let offset = data as usize + 4 + element * 4;
                    elements.push(read_u32(list_indices, offset, "list element")? as usize);
                }
                Value::List(elements)
            } else if matches!(type_id, 6 | 7 | 9 | 10 | 11 | 12 | 13 | 16 | 17 | 18) {
                Value::decode_field_data(type_id, data, field_data, block)?
            } else {
                Value::decode_inline(type_id, data)
                    .map_err(|_| err(format!("field {index} has unknown type {type_id}")))?
            };
            fields.push(Field {
                label,
                type_id,
                value,
            });
        }

        let mut structs = Vec::with_capacity(struct_count);
        let mut reference = Vec::with_capacity(struct_count);
        for index in 0..struct_count {
            let start = struct_offset + index * STRUCT_ENTRY_SIZE;
            let type_id = read_u32(bytes, start, "struct type")?;
            let data = read_u32(bytes, start + 4, "struct field reference")?;
            let count = read_u32(bytes, start + 8, "struct field count")?;
            reference.push((data, count));
            structs.push(Struct {
                type_id,
                fields: Vec::new(),
            });
        }
        for (index, (data, count)) in reference.into_iter().enumerate() {
            let mut indices = Vec::with_capacity(count as usize);
            match count {
                0 => {}
                1 => {
                    // Single field structs inline the field index in the data
                    // word instead of pointing into the field index array.
                    indices.push(usize::try_from(data).map_err(|_| {
                        err(format!("struct {index} has an invalid field reference"))
                    })?);
                }
                _ => {
                    let offset = data as usize;
                    if offset % 4 != 0 {
                        return Err(err(format!(
                            "struct {index} has unaligned field index offset {offset}"
                        )));
                    }
                    for element in 0..count as usize {
                        let value =
                            read_u32(field_indices, offset + element * 4, "struct field index")?
                                as usize;
                        indices.push(value);
                    }
                }
            }
            for field_index in indices {
                let field = fields.get(field_index).cloned().ok_or_else(|| {
                    err(format!(
                        "struct {index} references field {field_index}, but only {field_count} fields exist"
                    ))
                })?;
                structs[index].fields.push(field);
            }
        }

        // Validate list elements now that all structs are known, and remap
        // nothing else; this keeps the reader strict about damaged files.
        for structure in &structs {
            for field in &structure.fields {
                if let Value::List(elements) = &field.value {
                    if let Some(bad) = elements.iter().find(|index| **index >= structs.len()) {
                        return Err(err(format!(
                            "list references struct {bad}, but only {} structs exist",
                            structs.len()
                        )));
                    }
                }
            }
        }

        let root = structs
            .iter()
            .position(|structure| structure.type_id == u32::MAX)
            .unwrap_or(0);

        Ok(Self {
            version,
            labels,
            structs,
            root,
        })
    }

    /// Decodes only the fields named by `paths`, without materialising the
    /// rest of the tree.
    ///
    /// This is used for the savegame list, which needs a handful of values from
    /// a block that may contain tens of thousands of fields. The header and
    /// tables are validated exactly like [`Gff::parse`], but values for fields
    /// that are not requested are never decoded. A missing path yields `None`;
    /// malformed offsets or a bad target type still produce an error.
    #[allow(clippy::too_many_lines)] // the parser mirrors the on-disk layout
    pub fn selected_fields(
        block: &str,
        bytes: &[u8],
        paths: &[&str],
    ) -> Result<Vec<Option<Value>>> {
        let err = |message: String| Error::Malformed {
            block: block.to_owned(),
            message,
        };
        if bytes.len() < HEADER_SIZE {
            return Err(err(format!(
                "block is {} bytes, smaller than the {HEADER_SIZE} byte header",
                bytes.len()
            )));
        }
        if bytes[..4] != MAGIC {
            return Err(err(format!("bad magic {:?}", &bytes[..4])));
        }
        let mut version = [0_u8; 4];
        version.copy_from_slice(&bytes[4..8]);
        if &version[..2] != b"V3" {
            return Err(err(format!(
                "unsupported version {:?}",
                String::from_utf8_lossy(&version)
            )));
        }
        let word = |index: usize| -> Result<u32> {
            let offset = 8 + index * 4;
            bytes
                .get(offset..offset + 4)
                .and_then(|slice| slice.try_into().ok())
                .map(u32::from_le_bytes)
                .ok_or_else(|| err(format!("header word {index} is truncated")))
        };
        let struct_offset = word(0)? as usize;
        let struct_count = word(1)? as usize;
        let field_offset = word(2)? as usize;
        let field_count = word(3)? as usize;
        let label_offset = word(4)? as usize;
        let label_count = word(5)? as usize;
        let field_data_offset = word(6)? as usize;
        let field_data_size = word(7)? as usize;
        let field_indices_offset = word(8)? as usize;
        let field_indices_size = word(9)? as usize;
        let list_indices_offset = word(10)? as usize;
        let list_indices_size = word(11)? as usize;
        let end = |offset: usize, size: usize, what: &str| -> Result<()> {
            let end = offset
                .checked_add(size)
                .ok_or_else(|| err(format!("{what} overflows the address space")))?;
            if end > bytes.len() {
                return Err(err(format!("{what} ends at {end}, beyond the block")));
            }
            Ok(())
        };
        end(
            struct_offset,
            struct_count * STRUCT_ENTRY_SIZE,
            "struct table",
        )?;
        end(field_offset, field_count * FIELD_ENTRY_SIZE, "field table")?;
        end(label_offset, label_count * LABEL_SIZE, "label table")?;
        end(field_data_offset, field_data_size, "field data")?;
        end(field_indices_offset, field_indices_size, "field indices")?;
        end(list_indices_offset, list_indices_size, "list indices")?;
        let block_size = list_indices_offset
            .checked_add(list_indices_size)
            .ok_or_else(|| err("section offsets overflow".to_owned()))?;
        if block_size != bytes.len() {
            return Err(err(format!(
                "block size is {} but the sections end at {block_size}",
                bytes.len()
            )));
        }
        let mut labels = Vec::with_capacity(label_count);
        for index in 0..label_count {
            let start = label_offset + index * LABEL_SIZE;
            let slot = &bytes[start..start + LABEL_SIZE];
            let length = slot
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(LABEL_SIZE);
            labels.push(
                std::str::from_utf8(&slot[..length])
                    .map_err(|_| err(format!("label {index} is not valid UTF-8")))?
                    .to_owned(),
            );
        }
        let field_data = &bytes[field_data_offset..field_data_offset + field_data_size];
        let field_indices = &bytes[field_indices_offset..field_indices_offset + field_indices_size];
        let list_indices = &bytes[list_indices_offset..list_indices_offset + list_indices_size];
        let read_u32 = |buffer: &[u8], offset: usize, what: &str| -> Result<u32> {
            buffer
                .get(offset..offset + 4)
                .and_then(|slice| slice.try_into().ok())
                .map(u32::from_le_bytes)
                .ok_or_else(|| err(format!("{what} at offset {offset} is out of bounds")))
        };
        let fields: Vec<(u32, u32, u32)> = (0..field_count)
            .map(|index| {
                let start = field_offset + index * FIELD_ENTRY_SIZE;
                let type_id = read_u32(bytes, start, "field type")?;
                let label = read_u32(bytes, start + 4, "field label")?;
                if label as usize >= label_count {
                    return Err(err(format!("field {index} references label {label}")));
                }
                Ok((type_id, label, read_u32(bytes, start + 8, "field data")?))
            })
            .collect::<Result<_>>()?;
        let mut structures = Vec::with_capacity(struct_count);
        for index in 0..struct_count {
            let start = struct_offset + index * STRUCT_ENTRY_SIZE;
            let type_id = read_u32(bytes, start, "struct type")?;
            let data = read_u32(bytes, start + 4, "struct field reference")?;
            let count = read_u32(bytes, start + 8, "struct field count")?;
            structures.push((type_id, data, count));
        }
        let mut field_lists: Vec<Vec<usize>> = Vec::with_capacity(struct_count);
        for (index, (_, data, count)) in structures.iter().enumerate() {
            let mut indices = Vec::new();
            match count {
                0 => {}
                1 => indices.push(*data as usize),
                _ => {
                    let offset = *data as usize;
                    if offset % 4 != 0 {
                        return Err(err(format!("struct {index} has an unaligned field offset")));
                    }
                    for element in 0..*count as usize {
                        indices.push(read_u32(
                            field_indices,
                            offset + element * 4,
                            "struct field index",
                        )? as usize);
                    }
                }
            }
            for field in &indices {
                if *field >= fields.len() {
                    return Err(err(format!(
                        "struct {index} references field {field}, but only {} fields exist",
                        fields.len()
                    )));
                }
            }
            field_lists.push(indices);
        }
        let root = structures
            .iter()
            .position(|(type_id, _, _)| *type_id == u32::MAX)
            .unwrap_or(0);
        let mut selected = Vec::with_capacity(paths.len());
        for path in paths {
            let mut current = root;
            let mut value = None;
            let segments: Vec<&str> = path.split('.').filter(|part| !part.is_empty()).collect();
            if segments.is_empty() {
                selected.push(None);
                continue;
            }
            for (position, segment) in segments.iter().enumerate() {
                let Some(field_index) = field_lists.get(current).and_then(|list| {
                    list.iter()
                        .copied()
                        .find(|index| labels[fields[*index].1 as usize] == *segment)
                }) else {
                    break;
                };
                let (type_id, _, data) = fields[field_index];
                if position + 1 == segments.len() {
                    let decoded = if type_id == TYPE_LIST {
                        let count = read_u32(list_indices, data as usize, "list count")? as usize;
                        let mut elements = Vec::with_capacity(count.min(1 << 16));
                        for element in 0..count {
                            elements.push(read_u32(
                                list_indices,
                                data as usize + 4 + element * 4,
                                "list element",
                            )? as usize);
                        }
                        Value::List(elements)
                    } else if matches!(type_id, 6 | 7 | 9 | 10 | 11 | 12 | 13 | 16 | 17 | 18) {
                        Value::decode_field_data(type_id, data, field_data, block)
                            .map_err(|_| err(format!("field {path} is malformed")))?
                    } else {
                        Value::decode_inline(type_id, data)
                            .map_err(|_| err(format!("field {path} has unknown type {type_id}")))?
                    };
                    value = Some(decoded);
                } else {
                    if type_id != TYPE_STRUCT || data as usize >= field_lists.len() {
                        break;
                    }
                    current = data as usize;
                }
            }
            selected.push(value);
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
            .map(|index| index as u32)
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
        self.labels.push(name.to_owned());
        Ok((self.labels.len() - 1) as u32)
    }

    /// Returns the root struct.
    #[must_use]
    pub fn root(&self) -> &Struct {
        &self.structs[self.root]
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
    pub fn remove(&mut self, path: impl AsRef<str>) -> Result<Field> {
        let path = FieldPath::parse(path.as_ref())?;
        let segments = path.segments();
        let (leaf, _) = segments.split_last().ok_or_else(|| Error::FieldNotFound {
            path: path.to_string(),
        })?;
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
        Ok(&self.structs[struct_index].fields[field_index])
    }

    /// Looks up a field by path, mutably.
    pub fn get_mut(&mut self, path: impl AsRef<str>) -> Result<&mut Field> {
        let path = FieldPath::parse(path.as_ref())?;
        let (struct_index, field_index) =
            self.resolve(&path).ok_or_else(|| Error::FieldNotFound {
                path: path.to_string(),
            })?;
        Ok(&mut self.structs[struct_index].fields[field_index])
    }

    /// Replaces the value of a field. The new value has to have the same kind
    /// as the old one, which keeps the on-disk type tag stable.
    pub fn set(&mut self, path: impl AsRef<str>, value: Value) -> Result<()> {
        let path = path.as_ref();
        let field = self.get_mut(path)?;
        if field.type_id != value.type_id() {
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
        let label = self.intern_label(label)?;
        let structure = self
            .structs
            .get_mut(struct_index)
            .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?;
        if structure.fields.iter().any(|field| field.label == label) {
            return Err(Error::Corrupt(format!(
                "struct {struct_index} already has a field named {label:?}"
            )));
        }
        structure.fields.push(Field {
            label,
            type_id: value.type_id(),
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
        for (index, label) in self.labels.iter().enumerate() {
            if label.len() > LABEL_MAX_LEN || label.as_bytes().contains(&0) {
                return Err(err(format!(
                    "label {index} ({label:?}) does not fit into {LABEL_MAX_LEN} bytes"
                )));
            }
        }
        for (index, structure) in self.structs.iter().enumerate() {
            let mut seen = std::collections::HashSet::new();
            for field in &structure.fields {
                if field.label as usize >= self.labels.len() {
                    return Err(err(format!(
                        "struct {index} uses label {} which does not exist",
                        field.label
                    )));
                }
                if !seen.insert(field.label) {
                    return Err(err(format!(
                        "struct {index} contains the label {:?} twice",
                        self.labels[field.label as usize]
                    )));
                }
                let compatible = field.type_id == field.value.type_id()
                    || (matches!(field.type_id, 12 | 18)
                        && matches!(field.value, Value::LocString(_)));
                if !compatible {
                    return Err(err(format!(
                        "struct {index} field {:?} has type tag {} but holds {}",
                        self.labels[field.label as usize],
                        field.type_id,
                        field.value.type_name()
                    )));
                }
                match &field.value {
                    Value::Struct(target) if *target >= self.structs.len() => {
                        return Err(err(format!(
                            "struct {index} points at struct {target}, out of bounds"
                        )));
                    }
                    Value::List(elements) => {
                        for element in elements {
                            if *element >= self.structs.len() {
                                return Err(err(format!(
                                    "struct {index} contains a list with out of bounds struct {element}"
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
