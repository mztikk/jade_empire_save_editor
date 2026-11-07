//! Strict decoding of a block's header, tables and index sections.
//!
//! Both [`crate::Gff::parse`] and [`crate::Gff::selected_fields`] use this
//! module. Keeping one implementation means the cheap metadata path cannot
//! drift from the full parser: every block is validated the same way, only the
//! field values are decoded on demand.

use std::collections::HashSet;

use crate::{
    Error, FIELD_ENTRY_SIZE, HEADER_SIZE, LABEL_SIZE, MAGIC, Result, STRUCT_ENTRY_SIZE, TYPE_LIST,
    TYPE_STRUCT,
};

/// A section of the block, as described by the header.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Section {
    /// Start of the section in the block.
    pub(crate) offset: usize,
    /// Length of the section in bytes.
    pub(crate) size: usize,
}

/// One entry of the field table, before its value is decoded.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldEntry {
    /// Type tag from the field table.
    pub(crate) type_id: u32,
    /// Index into [`Tables::labels`].
    pub(crate) label: u32,
    /// The 32 bit data word from the field table.
    pub(crate) data: u32,
}

/// One entry of the struct table.
#[derive(Debug, Clone)]
pub(crate) struct StructEntry {
    /// Type id from the struct table.
    pub(crate) type_id: u32,
    /// Indices into [`Tables::fields`], in storage order.
    pub(crate) fields: Vec<usize>,
}

/// The validated tables of a GFF block.
#[derive(Debug, Clone)]
pub(crate) struct Tables {
    /// Version tag from the block header.
    pub(crate) version: [u8; 4],
    /// Label table.
    pub(crate) labels: Vec<String>,
    /// Field table.
    pub(crate) fields: Vec<FieldEntry>,
    /// Struct table.
    pub(crate) structs: Vec<StructEntry>,
    /// Index of the root struct.
    pub(crate) root: usize,
    /// Field data section.
    pub(crate) field_data: Section,
    /// List index section.
    pub(crate) list_indices: Section,
}

impl Tables {
    /// Parses and strictly validates a block's header and tables.
    #[allow(clippy::too_many_lines)] // the checks mirror the on-disk layout
    pub(crate) fn parse(block: &str, bytes: &[u8]) -> Result<Self> {
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

        // The writer emits this exact canonical chain and every one of the 962
        // retail blocks follows it. Enforcing it prevents overlapping sections
        // from being interpreted as a different, silently truncated tree.
        let size = |count: usize, entry: usize, what: &str| -> Result<usize> {
            count
                .checked_mul(entry)
                .ok_or_else(|| err(format!("{what} size overflows the address space")))
        };
        let struct_size = size(struct_count, STRUCT_ENTRY_SIZE, "struct table")?;
        let field_size = size(field_count, FIELD_ENTRY_SIZE, "field table")?;
        let label_size = size(label_count, LABEL_SIZE, "label table")?;

        if struct_offset != HEADER_SIZE {
            return Err(err(format!(
                "struct table starts at {struct_offset}, expected {HEADER_SIZE}"
            )));
        }
        let expected_field_offset = HEADER_SIZE
            .checked_add(struct_size)
            .ok_or_else(|| err("struct table ends beyond the address space".to_owned()))?;
        if field_offset != expected_field_offset {
            return Err(err(format!(
                "field table starts at {field_offset}, expected {expected_field_offset}"
            )));
        }
        let expected_label_offset = field_offset
            .checked_add(field_size)
            .ok_or_else(|| err("field table ends beyond the address space".to_owned()))?;
        if label_offset != expected_label_offset {
            return Err(err(format!(
                "label table starts at {label_offset}, expected {expected_label_offset}"
            )));
        }
        let expected_field_data_offset = label_offset
            .checked_add(label_size)
            .ok_or_else(|| err("label table ends beyond the address space".to_owned()))?;
        if field_data_offset != expected_field_data_offset {
            return Err(err(format!(
                "field data starts at {field_data_offset}, expected {expected_field_data_offset}"
            )));
        }
        let expected_field_indices_offset = field_data_offset
            .checked_add(field_data_size)
            .ok_or_else(|| err("field data ends beyond the address space".to_owned()))?;
        if field_indices_offset != expected_field_indices_offset {
            return Err(err(format!(
                "field indices start at {field_indices_offset}, expected {expected_field_indices_offset}"
            )));
        }
        let expected_list_indices_offset = field_indices_offset
            .checked_add(field_indices_size)
            .ok_or_else(|| err("field indices end beyond the address space".to_owned()))?;
        if list_indices_offset != expected_list_indices_offset {
            return Err(err(format!(
                "list indices start at {list_indices_offset}, expected {expected_list_indices_offset}"
            )));
        }
        let block_end = list_indices_offset
            .checked_add(list_indices_size)
            .ok_or_else(|| err("list indices end beyond the address space".to_owned()))?;
        if block_end != bytes.len() {
            return Err(err(format!(
                "block size is {} bytes but the sections describe {block_end} bytes",
                bytes.len()
            )));
        }

        let field_data = Section {
            offset: field_data_offset,
            size: field_data_size,
        };
        let list_indices = Section {
            offset: list_indices_offset,
            size: list_indices_size,
        };

        let mut labels = Vec::with_capacity(label_count);
        let mut seen_labels = HashSet::with_capacity(label_count);
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
            if !seen_labels.insert(name.clone()) {
                return Err(err(format!("label table contains {name:?} twice")));
            }
            labels.push(name);
        }

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
            fields.push(FieldEntry {
                type_id,
                label,
                data,
            });
        }

        let field_indices_slice =
            &bytes[field_indices_offset..field_indices_offset + field_indices_size];
        let list_indices_slice =
            &bytes[list_indices.offset..list_indices.offset + list_indices.size];

        let mut structs = Vec::with_capacity(struct_count);
        let mut field_index_ranges: Vec<(usize, usize)> = Vec::new();
        for index in 0..struct_count {
            let start = struct_offset + index * STRUCT_ENTRY_SIZE;
            let type_id = read_u32(bytes, start, "struct type")?;
            let data = read_u32(bytes, start + 4, "struct field reference")?;
            let count = read_u32(bytes, start + 8, "struct field count")? as usize;
            // A struct can never reference more fields than the field table
            // contains. Checking this before `with_capacity` is what keeps a
            // 68-byte hostile block from requesting tens of gigabytes.
            if count > field_count {
                return Err(err(format!(
                    "struct {index} has {count} fields, but only {field_count} exist"
                )));
            }
            let mut indices = Vec::with_capacity(count);
            match count {
                0 => {}
                1 => {
                    // Single field structs inline the field index in the data
                    // word instead of pointing into the field index array.
                    let field_index = usize::try_from(data).map_err(|_| {
                        err(format!("struct {index} has an invalid field reference"))
                    })?;
                    if field_index >= field_count {
                        return Err(err(format!(
                            "struct {index} references field {field_index}, but only {field_count} exist"
                        )));
                    }
                    indices.push(field_index);
                }
                _ => {
                    let offset = data as usize;
                    if !offset.is_multiple_of(4) {
                        return Err(err(format!(
                            "struct {index} has unaligned field index offset {offset}"
                        )));
                    }
                    let bytes_needed = count
                        .checked_mul(4)
                        .ok_or_else(|| err(format!("struct {index} field indices overflow")))?;
                    let end = offset.checked_add(bytes_needed).ok_or_else(|| {
                        err(format!("struct {index} field indices overflow the section"))
                    })?;
                    if end > field_indices_slice.len() {
                        return Err(err(format!(
                            "struct {index} field indices end at {end}, beyond the {} byte section",
                            field_indices_slice.len()
                        )));
                    }
                    let mut seen = HashSet::with_capacity(count);
                    for element in 0..count {
                        let field_index = read_u32(
                            field_indices_slice,
                            offset + element * 4,
                            "struct field index",
                        )? as usize;
                        if field_index >= field_count {
                            return Err(err(format!(
                                "struct {index} references field {field_index}, but only {field_count} exist"
                            )));
                        }
                        if !seen.insert(field_index) {
                            return Err(err(format!(
                                "struct {index} references field {field_index} twice"
                            )));
                        }
                        indices.push(field_index);
                    }
                    field_index_ranges.push((offset, end));
                }
            }
            structs.push(StructEntry {
                type_id,
                fields: indices,
            });
        }

        // The field-index section is a tiling of all multi-field structs. Any
        // gap, overlap, duplicate or trailing byte means the block does not
        // match the format.
        field_index_ranges.sort_unstable_by_key(|(start, _)| *start);
        let mut cursor = 0_usize;
        for (start, end) in field_index_ranges {
            if start != cursor {
                return Err(err(format!(
                    "field index ranges are not contiguous at {start} (expected {cursor})"
                )));
            }
            cursor = end;
        }
        if cursor != field_indices_size {
            return Err(err(format!(
                "field index section is {field_indices_size} bytes but the structs describe {cursor}"
            )));
        }

        for (index, field) in fields.iter().enumerate() {
            match field.type_id {
                TYPE_STRUCT => {
                    if field.data as usize >= struct_count {
                        return Err(err(format!(
                            "field {index} references struct {}, but only {struct_count} structs exist",
                            field.data
                        )));
                    }
                }
                TYPE_LIST => {
                    validate_list(block, field.data, struct_count, list_indices_slice)?;
                }
                _ => {}
            }
        }

        let mut roots = structs
            .iter()
            .enumerate()
            .filter(|(_, structure)| structure.type_id == u32::MAX);
        let root = roots
            .next()
            .map(|(index, _)| index)
            .ok_or_else(|| err("block has no root struct (type 0xffff_ffff)".to_owned()))?;
        if roots.next().is_some() {
            return Err(err(
                "block has more than one root struct (type 0xffff_ffff)".to_owned(),
            ));
        }

        Ok(Self {
            version,
            labels,
            fields,
            structs,
            root,
            field_data,
            list_indices,
        })
    }

    /// The field data section of a block.
    pub(crate) fn field_data<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        &bytes[self.field_data.offset..self.field_data.offset + self.field_data.size]
    }

    /// The list index section of a block.
    pub(crate) fn list_indices<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        &bytes[self.list_indices.offset..self.list_indices.offset + self.list_indices.size]
    }
}

/// Validates one list reference (count plus all struct indices).
pub(crate) fn validate_list(
    block: &str,
    data: u32,
    struct_count: usize,
    list_indices: &[u8],
) -> Result<()> {
    let err = |message: String| Error::Malformed {
        block: block.to_owned(),
        message,
    };
    let start = data as usize;
    let count = list_indices
        .get(start..start + 4)
        .and_then(|slice| slice.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| err(format!("list count at offset {start} is out of bounds")))?
        as usize;
    let bytes_needed = count
        .checked_mul(4)
        .ok_or_else(|| err("list element count overflows the address space".to_owned()))?;
    let end = start
        .checked_add(4)
        .and_then(|value| value.checked_add(bytes_needed))
        .ok_or_else(|| err("list end overflows the address space".to_owned()))?;
    if end > list_indices.len() {
        return Err(err(format!(
            "list at {start} with {count} elements ends at {end}, beyond the {} byte section",
            list_indices.len()
        )));
    }
    for element in 0..count {
        let offset = start + 4 + element * 4;
        let target = list_indices
            .get(offset..offset + 4)
            .and_then(|slice| slice.try_into().ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| err(format!("list element at {offset} is out of bounds")))?
            as usize;
        if target >= struct_count {
            return Err(err(format!(
                "list at {start} references struct {target}, but only {struct_count} structs exist"
            )));
        }
    }
    Ok(())
}

/// Reads a list value that was already validated by [`Tables::parse`].
pub(crate) fn read_list(list_indices: &[u8], data: u32) -> Result<Vec<usize>> {
    let start = data as usize;
    let count = list_indices
        .get(start..start + 4)
        .and_then(|slice| slice.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| Error::Corrupt("list count is out of bounds".to_owned()))?
        as usize;
    let mut elements = Vec::with_capacity(count);
    for element in 0..count {
        let offset = start + 4 + element * 4;
        let target = list_indices
            .get(offset..offset + 4)
            .and_then(|slice| slice.try_into().ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| Error::Corrupt("list element is out of bounds".to_owned()))?;
        elements.push(target as usize);
    }
    Ok(elements)
}
