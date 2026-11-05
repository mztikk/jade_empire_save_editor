//! Dotted field paths.

use crate::error::{Error, Result};

/// One step of a field path, optionally indexing into a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Field label.
    pub name: String,
    /// Optional list index.
    pub index: Option<usize>,
}

/// A parsed path such as `Stats.Focus.STAT_CUR` or `ItemList[0].Count`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPath {
    segments: Vec<Segment>,
}

impl FieldPath {
    /// Parses a dotted path.
    pub fn parse(path: &str) -> Result<Self> {
        if path.is_empty() {
            return Err(Error::FieldNotFound {
                path: String::new(),
            });
        }
        let mut segments = Vec::new();
        for raw in path.split('.') {
            if raw.is_empty() {
                return Err(Error::FieldNotFound {
                    path: path.to_owned(),
                });
            }
            let (name, index) = match raw.split_once('[') {
                Some((name, rest)) => {
                    let number = rest.strip_suffix(']').ok_or_else(|| Error::FieldNotFound {
                        path: path.to_owned(),
                    })?;
                    let index = number.parse::<usize>().map_err(|_| Error::FieldNotFound {
                        path: path.to_owned(),
                    })?;
                    (name, Some(index))
                }
                None => (raw, None),
            };
            if name.is_empty() {
                return Err(Error::FieldNotFound {
                    path: path.to_owned(),
                });
            }
            segments.push(Segment {
                name: name.to_owned(),
                index,
            });
        }
        Ok(Self { segments })
    }

    /// The individual path segments.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// The path without its final segment, used to resolve the parent struct.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.segments.len() <= 1 {
            return None;
        }
        Some(Self {
            segments: self.segments[..self.segments.len() - 1].to_vec(),
        })
    }

    /// The label of the final segment.
    #[must_use]
    pub fn field_name(&self) -> &str {
        self.segments
            .last()
            .map_or("", |segment| segment.name.as_str())
    }
}

impl std::fmt::Display for FieldPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (position, segment) in self.segments.iter().enumerate() {
            if position > 0 {
                formatter.write_str(".")?;
            }
            formatter.write_str(&segment.name)?;
            if let Some(index) = segment.index {
                write!(formatter, "[{index}]")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_path() {
        let path = FieldPath::parse("Stats.Gold").expect("path");
        assert_eq!(path.segments().len(), 2);
        assert_eq!(path.segments()[0].name, "Stats");
        assert_eq!(path.field_name(), "Gold");
    }

    #[test]
    fn parses_list_index() {
        let path = FieldPath::parse("ItemList[12].Count").expect("path");
        assert_eq!(path.segments()[0].index, Some(12));
        assert_eq!(path.to_string(), "ItemList[12].Count");
    }

    #[test]
    fn rejects_empty_path() {
        assert!(FieldPath::parse("").is_err());
    }

    #[test]
    fn rejects_malformed_index() {
        assert!(FieldPath::parse("ItemList[x]").is_err());
        assert!(FieldPath::parse("ItemList[1").is_err());
    }

    #[test]
    fn rejects_double_dots() {
        assert!(FieldPath::parse("Stats..Gold").is_err());
    }

    #[test]
    fn rejects_a_missing_segment_name() {
        assert!(FieldPath::parse("[0]").is_err());
    }

    #[test]
    fn parent_is_none_for_a_single_segment() {
        assert!(FieldPath::parse("Gold").expect("path").parent().is_none());
        let parent = FieldPath::parse("Stats.Focus.STAT_CUR")
            .expect("path")
            .parent()
            .expect("parent");
        assert_eq!(parent.to_string(), "Stats.Focus");
    }
}
