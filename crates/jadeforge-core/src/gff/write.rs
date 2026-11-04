//! Serializer for the GFF variant used by Jade Empire savegames.
//!
//! The writer emits a canonical layout: structs keep their order, fields keep
//! the order they have inside their struct, and the field data section is
//! written in field order. Identical payloads are stored once so that the
//! output does not grow when a block is rewritten unchanged.

use std::collections::HashMap;

use super::{FIELD_ENTRY_SIZE, Gff, HEADER_SIZE, LABEL_SIZE, STRUCT_ENTRY_SIZE, Value};
use crate::error::{Error, Result};

impl Gff {
    /// Serializes the block back into its on-disk representation.
    #[allow(clippy::too_many_lines)] // the writer mirrors the on-disk layout
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate("write")?;

        let struct_count = self.structs.len();
        let field_count = self.field_count();
        let label_count = self.labels.len();

        // Flatten all fields in struct order and remember the range that every
        // struct occupies inside the flattened array.
        let mut flat_fields: Vec<&super::Field> = Vec::with_capacity(field_count);
        let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(struct_count);
        for structure in &self.structs {
            let start = flat_fields.len();
            flat_fields.extend(structure.fields.iter());
            ranges.push((start, structure.fields.len()));
        }
        debug_assert_eq!(flat_fields.len(), field_count);

        let struct_offset = HEADER_SIZE;
        let field_offset = struct_offset + struct_count * STRUCT_ENTRY_SIZE;
        let label_offset = field_offset + field_count * FIELD_ENTRY_SIZE;
        let field_data_offset = label_offset + label_count * LABEL_SIZE;

        // Field data section, de-duplicated by (type, payload).
        let mut field_data: Vec<u8> = Vec::new();
        let mut field_data_seen: HashMap<(u32, Vec<u8>), u32> = HashMap::new();
        let mut data_words: Vec<u32> = vec![0; field_count];
        for (index, field) in flat_fields.iter().enumerate() {
            if let Value::List(_) = field.value {
                continue; // filled in below
            }
            if let Some(payload) = field.value.encode() {
                let key = (field.type_id, payload.clone());
                data_words[index] = if let Some(existing) = field_data_seen.get(&key) {
                    *existing
                } else {
                    let offset = to_offset(field_data.len(), "field data")?;
                    field_data.extend_from_slice(&payload);
                    field_data_seen.insert(key, offset);
                    offset
                };
            } else {
                data_words[index] = field.value.encode_inline().ok_or_else(|| {
                    Error::Corrupt(format!("field {} cannot be encoded", field.label))
                })?;
            }
        }

        // Field index array: one entry per field of every multi-field struct.
        let mut field_indices: Vec<u8> = Vec::new();
        let mut struct_refs: Vec<u32> = vec![0; struct_count];
        let mut struct_counts: Vec<u32> = Vec::with_capacity(struct_count);
        for (index, (start, count)) in ranges.iter().enumerate() {
            struct_counts.push(to_offset(*count, "struct field count")?);
            match count {
                0 => {}
                1 => {
                    // Single field structs inline the field index in the data
                    // word instead of pointing into the field index array.
                    struct_refs[index] = to_offset(*start, "field index")?;
                }
                _ => {
                    struct_refs[index] = to_offset(field_indices.len(), "field index array")?;
                    for offset in 0..*count {
                        let field_index = to_offset(start + offset, "field index")?;
                        field_indices.extend_from_slice(&field_index.to_le_bytes());
                    }
                }
            }
        }

        // List index array.
        let mut list_indices: Vec<u8> = Vec::new();
        let mut list_seen: HashMap<Vec<u8>, u32> = HashMap::new();
        for (index, field) in flat_fields.iter().enumerate() {
            let Value::List(elements) = &field.value else {
                continue;
            };
            let mut payload = Vec::with_capacity(elements.len() * 4 + 4);
            payload.extend_from_slice(&to_offset(elements.len(), "list length")?.to_le_bytes());
            for element in elements {
                payload.extend_from_slice(&to_offset(*element, "list element")?.to_le_bytes());
            }
            data_words[index] = if let Some(existing) = list_seen.get(&payload) {
                *existing
            } else {
                let offset = to_offset(list_indices.len(), "list index array")?;
                list_indices.extend_from_slice(&payload);
                list_seen.insert(payload, offset);
                offset
            };
        }

        let field_indices_offset = field_data_offset + field_data.len();
        let list_indices_offset = field_indices_offset + field_indices.len();
        let total = list_indices_offset + list_indices.len();

        let mut out = vec![0_u8; total];
        out[..4].copy_from_slice(&super::MAGIC);
        out[4..8].copy_from_slice(&self.version);

        let header = [
            struct_offset,
            struct_count,
            field_offset,
            field_count,
            label_offset,
            label_count,
            field_data_offset,
            field_data.len(),
            field_indices_offset,
            field_indices.len(),
            list_indices_offset,
            list_indices.len(),
        ];
        for (index, value) in header.iter().enumerate() {
            let offset = 8 + index * 4;
            out[offset..offset + 4]
                .copy_from_slice(&to_offset(*value, "header field")?.to_le_bytes());
        }

        for (index, field) in flat_fields.iter().enumerate() {
            let offset = field_offset + index * FIELD_ENTRY_SIZE;
            out[offset..offset + 4].copy_from_slice(&field.type_id.to_le_bytes());
            out[offset + 4..offset + 8].copy_from_slice(&field.label.to_le_bytes());
            out[offset + 8..offset + 12].copy_from_slice(&data_words[index].to_le_bytes());
        }

        for (index, structure) in self.structs.iter().enumerate() {
            let offset = struct_offset + index * STRUCT_ENTRY_SIZE;
            out[offset..offset + 4].copy_from_slice(&structure.type_id.to_le_bytes());
            out[offset + 4..offset + 8].copy_from_slice(&struct_refs[index].to_le_bytes());
            out[offset + 8..offset + 12].copy_from_slice(&struct_counts[index].to_le_bytes());
        }

        for (index, label) in self.labels.iter().enumerate() {
            let offset = label_offset + index * LABEL_SIZE;
            out[offset..offset + label.len()].copy_from_slice(label.as_bytes());
        }

        out[field_data_offset..field_data_offset + field_data.len()].copy_from_slice(&field_data);
        out[field_indices_offset..field_indices_offset + field_indices.len()]
            .copy_from_slice(&field_indices);
        out[list_indices_offset..list_indices_offset + list_indices.len()]
            .copy_from_slice(&list_indices);

        Ok(out)
    }
}

fn to_offset(value: usize, what: &str) -> Result<u32> {
    u32::try_from(value)
        .map_err(|_| Error::Corrupt(format!("{what} is too large for the file format")))
}
