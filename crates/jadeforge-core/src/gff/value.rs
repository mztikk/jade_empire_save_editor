//! Values stored inside GFF fields.
//!
//! Jade Empire stores savegame payloads in a variant of `BioWare`'s *generic file
//! format* (GFF). Every field owns a type tag, a label and a value. This module
//! models those values and knows how to encode/decode the byte representation
//! used in the "field data" section of the file.

use crate::error::{Error, Result};

/// A localized string: either a reference into the game's talk table, inline
/// text, or both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocString {
    /// Index into `dialog.tlk`, if the text is localized.
    pub strref: Option<u32>,
    /// Inline text, if the string is not (only) localized.
    pub text: Option<String>,
}

impl LocString {
    /// Creates a string that only points into the talk table.
    #[must_use]
    pub fn from_strref(strref: u32) -> Self {
        Self {
            strref: Some(strref),
            text: None,
        }
    }

    /// Creates a string with inline text.
    #[must_use]
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            strref: None,
            text: Some(text.into()),
        }
    }
}

/// A single value inside a GFF field.
#[derive(Debug, Clone)]
pub enum Value {
    /// 8 bit unsigned integer, stored inline.
    Byte(u8),
    /// 8 bit signed integer, stored inline.
    Char(i8),
    /// 16 bit unsigned integer, stored inline.
    Word(u16),
    /// 16 bit signed integer, stored inline.
    Short(i16),
    /// 32 bit unsigned integer, stored inline.
    DWord(u32),
    /// 32 bit signed integer, stored inline.
    Int(i32),
    /// 64 bit unsigned integer, stored in the field data section.
    DWord64(u64),
    /// 64 bit signed integer, stored in the field data section.
    Int64(i64),
    /// IEEE-754 single precision float, stored inline.
    Float(f32),
    /// IEEE-754 double precision float, stored in the field data section.
    Double(f64),
    /// A C string.
    String(String),
    /// A resource reference (file name without extension).
    ResRef(String),
    /// A localized string.
    LocString(LocString),
    /// An opaque blob.
    Void(Vec<u8>),
    /// Index of a sub-struct in [`crate::gff::Gff::structs`].
    Struct(usize),
    /// Indices of sub-structs in [`crate::gff::Gff::structs`].
    List(Vec<usize>),
    /// Four floats, used for camera and object orientations.
    Quaternion([f32; 4]),
    /// Three floats, used for positions.
    Vector3([f32; 3]),
}

impl Value {
    /// Human readable name of the value's kind, matching the type tags of the
    /// file format.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Byte(_) => "byte",
            Self::Char(_) => "char",
            Self::Word(_) => "word",
            Self::Short(_) => "short",
            Self::DWord(_) => "dword",
            Self::Int(_) => "int",
            Self::DWord64(_) => "dword64",
            Self::Int64(_) => "int64",
            Self::Float(_) => "float",
            Self::Double(_) => "double",
            Self::String(_) => "string",
            Self::ResRef(_) => "resref",
            Self::LocString(_) => "locstring",
            Self::Void(_) => "void",
            Self::Struct(_) => "struct",
            Self::List(_) => "list",
            Self::Quaternion(_) => "quaternion",
            Self::Vector3(_) => "vector3",
        }
    }

    /// The numeric type tag used by the on-disk format.
    #[must_use]
    pub fn type_id(&self) -> u32 {
        match self {
            Self::Byte(_) => 0,
            Self::Char(_) => 1,
            Self::Word(_) => 2,
            Self::Short(_) => 3,
            Self::DWord(_) => 4,
            Self::Int(_) => 5,
            Self::DWord64(_) => 6,
            Self::Int64(_) => 7,
            Self::Float(_) => 8,
            Self::Double(_) => 9,
            Self::String(_) => 10,
            Self::ResRef(_) => 11,
            Self::LocString(_) => 18,
            Self::Void(_) => 13,
            Self::Struct(_) => 14,
            Self::List(_) => 15,
            Self::Quaternion(_) => 16,
            Self::Vector3(_) => 17,
        }
    }

    /// Returns the value as an unsigned 32 bit integer when it is numeric.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            Self::Byte(v) => Some(u64::from(v)),
            Self::Word(v) => Some(u64::from(v)),
            Self::DWord(v) => Some(u64::from(v)),
            Self::DWord64(v) => Some(v),
            Self::Char(v) => Some(v as u64),
            Self::Short(v) => Some(v as u64),
            Self::Int(v) => Some(v as u64),
            _ => None,
        }
    }

    /// Returns the value as a signed 64 bit integer when it is numeric.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Self::Byte(v) => Some(i64::from(v)),
            Self::Word(v) => Some(i64::from(v)),
            Self::DWord(v) => Some(i64::from(v)),
            Self::DWord64(v) => Some(v as i64),
            Self::Char(v) => Some(i64::from(v)),
            Self::Short(v) => Some(i64::from(v)),
            Self::Int(v) => Some(i64::from(v)),
            Self::Int64(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the value as a float when it is numeric.
    #[must_use]
    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            Self::Float(v) => Some(v),
            Self::Double(v) => Some(v as f32),
            _ => self.as_i64().map(|v| v as f32),
        }
    }

    /// Returns the value as a string slice when it is text.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) | Self::ResRef(v) => Some(v),
            Self::LocString(v) => v.text.as_deref(),
            _ => None,
        }
    }

    /// Returns the struct index when this value references a struct.
    #[must_use]
    pub fn as_struct(&self) -> Option<usize> {
        match *self {
            Self::Struct(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the struct indices when this value is a list.
    #[must_use]
    pub fn as_list(&self) -> Option<&[usize]> {
        match self {
            Self::List(v) => Some(v),
            _ => None,
        }
    }

    /// Mutable access to the struct indices of a list.
    pub fn as_list_mut(&mut self) -> Option<&mut Vec<usize>> {
        match self {
            Self::List(v) => Some(v),
            _ => None,
        }
    }

    /// True when the value is stored in the field data section instead of being
    /// packed into the 32 bit data word of the field table.
    #[must_use]
    pub fn is_in_field_data(&self) -> bool {
        matches!(
            self,
            Self::DWord64(_)
                | Self::Int64(_)
                | Self::Double(_)
                | Self::String(_)
                | Self::ResRef(_)
                | Self::LocString(_)
                | Self::Void(_)
                | Self::Quaternion(_)
                | Self::Vector3(_)
        )
    }

    /// Encodes the value into the representation used in the field data
    /// section. Returns `None` for values that are stored inline or that
    /// reference other structures.
    #[must_use]
    pub fn encode(&self) -> Option<Vec<u8>> {
        Some(match self {
            Self::DWord64(v) => v.to_le_bytes().to_vec(),
            Self::Int64(v) => v.to_le_bytes().to_vec(),
            Self::Double(v) => v.to_le_bytes().to_vec(),
            Self::String(v) => encode_u32_string(v),
            Self::ResRef(v) => {
                let bytes = v.as_bytes();
                let mut out = Vec::with_capacity(bytes.len() + 1);
                // ResRefs are length prefixed with a single byte.
                out.push(u8::try_from(bytes.len()).unwrap_or(u8::MAX));
                out.extend_from_slice(bytes);
                out
            }
            Self::LocString(v) => {
                let mut out = Vec::new();
                let mut body = Vec::new();
                match v.strref {
                    Some(strref) => body.extend_from_slice(&strref.to_le_bytes()),
                    None => body.extend_from_slice(&u32::MAX.to_le_bytes()),
                }
                if let Some(text) = &v.text {
                    body.extend_from_slice(&encode_u32_string(text));
                }
                let total = u32::try_from(body.len()).ok()?;
                out.extend_from_slice(&total.to_le_bytes());
                out.extend_from_slice(&body);
                out
            }
            Self::Void(v) => {
                let mut out = Vec::with_capacity(v.len() + 4);
                let len = u32::try_from(v.len()).ok()?;
                out.extend_from_slice(&len.to_le_bytes());
                out.extend_from_slice(v);
                out
            }
            Self::Quaternion(v) => v.iter().flat_map(|f| f.to_le_bytes()).collect(),
            Self::Vector3(v) => v.iter().flat_map(|f| f.to_le_bytes()).collect(),
            _ => return None,
        })
    }

    /// Decodes a value of the given field type from the field data section.
    #[allow(clippy::too_many_lines)] // one arm per type keeps the decoding easy to audit
    ///
    /// `read` gives access to a region of the field data; `offset` is the
    /// offset stored in the field table entry.
    pub(crate) fn decode_field_data(
        type_id: u32,
        offset: u32,
        data: &[u8],
        block: &str,
    ) -> Result<Self> {
        let start = offset as usize;
        let take = |len: usize| -> Result<&[u8]> {
            data.get(start..start + len).ok_or_else(|| Error::Gff {
                block: block.to_owned(),
                message: format!(
                    "field data offset {start} with length {len} (type {type_id}) exceeds section size {}",
                    data.len()
                ),
            })
        };
        Ok(match type_id {
            6 => Self::DWord64(u64::from_le_bytes(take(8)?.try_into().expect("8 bytes"))),
            7 => Self::Int64(i64::from_le_bytes(take(8)?.try_into().expect("8 bytes"))),
            9 => Self::Double(f64::from_le_bytes(take(8)?.try_into().expect("8 bytes"))),
            10 => {
                let len = u32::from_le_bytes(take(4)?.try_into().expect("4 bytes")) as usize;
                let bytes = data
                    .get(start + 4..start + 4 + len)
                    .ok_or_else(|| Error::Gff {
                        block: block.to_owned(),
                        message: format!("string at {start} of length {len} is out of bounds"),
                    })?;
                Self::String(decode_text(bytes, block, start)?)
            }
            11 => {
                let len = usize::from(take(1)?[0]);
                let bytes = data
                    .get(start + 1..start + 1 + len)
                    .ok_or_else(|| Error::Gff {
                        block: block.to_owned(),
                        message: format!("resref at {start} of length {len} is out of bounds"),
                    })?;
                Self::ResRef(decode_text(bytes, block, start)?)
            }
            12 | 18 => {
                let total = u32::from_le_bytes(take(4)?.try_into().expect("4 bytes")) as usize;
                let inner = data
                    .get(start + 4..start + 4 + total)
                    .ok_or_else(|| Error::Gff {
                        block: block.to_owned(),
                        message: format!("localized string at {start} is out of bounds"),
                    })?;
                if inner.len() < 4 {
                    return Err(Error::Gff {
                        block: block.to_owned(),
                        message: format!("localized string at {start} is truncated"),
                    });
                }
                let strref = u32::from_le_bytes(inner[..4].try_into().expect("4 bytes"));
                let text = if inner.len() > 4 {
                    let len =
                        u32::from_le_bytes(inner[4..8].try_into().map_err(|_| Error::Gff {
                            block: block.to_owned(),
                            message: format!("localized string at {start} is truncated"),
                        })?) as usize;
                    let bytes = inner.get(8..8 + len).ok_or_else(|| Error::Gff {
                        block: block.to_owned(),
                        message: format!("localized string text at {start} is truncated"),
                    })?;
                    Some(decode_text(bytes, block, start)?)
                } else {
                    None
                };
                Self::LocString(LocString {
                    strref: (strref != u32::MAX).then_some(strref),
                    text,
                })
            }
            13 => {
                let len = u32::from_le_bytes(take(4)?.try_into().expect("4 bytes")) as usize;
                Self::Void(
                    data.get(start + 4..start + 4 + len)
                        .ok_or_else(|| Error::Gff {
                            block: block.to_owned(),
                            message: format!("blob at {start} of length {len} is out of bounds"),
                        })?
                        .to_vec(),
                )
            }
            16 => Self::Quaternion([
                f32::from_le_bytes(take(4)?.try_into().expect("4 bytes")),
                f32::from_le_bytes(
                    data.get(start + 4..start + 8)
                        .ok_or_else(|| Error::Gff {
                            block: block.to_owned(),
                            message: format!("quaternion at {start} is out of bounds"),
                        })?
                        .try_into()
                        .expect("4 bytes"),
                ),
                f32::from_le_bytes(
                    data.get(start + 8..start + 12)
                        .ok_or_else(|| Error::Gff {
                            block: block.to_owned(),
                            message: format!("quaternion at {start} is out of bounds"),
                        })?
                        .try_into()
                        .expect("4 bytes"),
                ),
                f32::from_le_bytes(
                    data.get(start + 12..start + 16)
                        .ok_or_else(|| Error::Gff {
                            block: block.to_owned(),
                            message: format!("quaternion at {start} is out of bounds"),
                        })?
                        .try_into()
                        .expect("4 bytes"),
                ),
            ]),
            17 => Self::Vector3([
                f32::from_le_bytes(take(4)?.try_into().expect("4 bytes")),
                f32::from_le_bytes(
                    data.get(start + 4..start + 8)
                        .ok_or_else(|| Error::Gff {
                            block: block.to_owned(),
                            message: format!("vector at {start} is out of bounds"),
                        })?
                        .try_into()
                        .expect("4 bytes"),
                ),
                f32::from_le_bytes(
                    data.get(start + 8..start + 12)
                        .ok_or_else(|| Error::Gff {
                            block: block.to_owned(),
                            message: format!("vector at {start} is out of bounds"),
                        })?
                        .try_into()
                        .expect("4 bytes"),
                ),
            ]),
            other => {
                return Err(Error::Gff {
                    block: block.to_owned(),
                    message: format!("unsupported field type {other}"),
                });
            }
        })
    }

    /// Decodes an inline (32 bit data word) field of the given type.
    pub(crate) fn decode_inline(type_id: u32, word: u32) -> Result<Self> {
        Ok(match type_id {
            0 => Self::Byte(word as u8),
            1 => Self::Char(word as u8 as i8),
            2 => Self::Word(word as u16),
            3 => Self::Short(word as u16 as i16),
            4 => Self::DWord(word),
            5 => Self::Int(word as i32),
            8 => Self::Float(f32::from_bits(word)),
            14 => Self::Struct(word as usize),
            other => return Err(Error::UnsupportedType(other)),
        })
    }

    /// The 32 bit data word for inline values.
    #[must_use]
    pub fn encode_inline(&self) -> Option<u32> {
        Some(match *self {
            Self::Byte(v) => u32::from(v),
            Self::Char(v) => u32::from(v as u8),
            Self::Word(v) => u32::from(v),
            Self::Short(v) => u32::from(v as u16),
            Self::DWord(v) => v,
            Self::Int(v) => v as u32,
            Self::Float(v) => v.to_bits(),
            Self::Struct(v) => u32::try_from(v).ok()?,
            _ => return None,
        })
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Byte(a), Self::Byte(b)) => a == b,
            (Self::Char(a), Self::Char(b)) => a == b,
            (Self::Word(a), Self::Word(b)) => a == b,
            (Self::Short(a), Self::Short(b)) => a == b,
            (Self::DWord(a), Self::DWord(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::DWord64(a), Self::DWord64(b)) => a == b,
            (Self::Int64(a), Self::Int64(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.to_bits() == b.to_bits(),
            (Self::Double(a), Self::Double(b)) => a.to_bits() == b.to_bits(),
            (Self::String(a), Self::String(b)) | (Self::ResRef(a), Self::ResRef(b)) => a == b,
            (Self::LocString(a), Self::LocString(b)) => a == b,
            (Self::Void(a), Self::Void(b)) => a == b,
            (Self::Struct(a), Self::Struct(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Quaternion(a), Self::Quaternion(b)) => {
                a.map(f32::to_bits) == b.map(f32::to_bits)
            }
            (Self::Vector3(a), Self::Vector3(b)) => a.map(f32::to_bits) == b.map(f32::to_bits),
            _ => false,
        }
    }
}

impl Eq for Value {}

fn encode_u32_string(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() + 4);
    out.extend_from_slice(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(bytes);
    out
}

fn decode_text(bytes: &[u8], block: &str, offset: usize) -> Result<String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| Error::Gff {
        block: block.to_owned(),
        message: format!("text at {offset} is not valid UTF-8"),
    })
}
