#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Regression tests for the strictness fixes: hostile counts, malformed
//! references, canonical layout and unrepresentable writer values.

use jadeforge_gff::{Error, Field, Gff, Struct, VERSION, Value};

fn empty_block() -> Gff {
    Gff {
        version: VERSION,
        labels: Vec::new(),
        structs: vec![Struct {
            type_id: u32::MAX,
            fields: Vec::new(),
        }],
        root: 0,
    }
}

fn single_field(label: &str, value: Value) -> Gff {
    let mut gff = empty_block();
    gff.add_field(0, label, value).expect("add field");
    gff
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("4 bytes"))
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn a_hostile_struct_field_count_is_rejected_before_allocating() {
    let bytes = single_field("A", Value::Byte(1))
        .to_bytes()
        .expect("serialize");
    let struct_offset = u32_at(&bytes, 8) as usize;
    let mut bad = bytes;
    put_u32(&mut bad, struct_offset + 8, u32::MAX);
    assert!(matches!(
        Gff::parse("count", &bad),
        Err(Error::Malformed { .. })
    ));
}

#[test]
fn dangling_struct_references_and_duplicate_field_indices_are_rejected() {
    let mut gff = empty_block();
    let child = gff.push_struct(0);
    gff.add_field(child, "X", Value::Byte(1)).expect("child");
    gff.add_field(0, "Ref", Value::Struct(child)).expect("ref");
    let bytes = gff.to_bytes().expect("serialize");
    let field_offset = u32_at(&bytes, 16) as usize;
    let field_count = u32_at(&bytes, 20) as usize;
    let reference_offset = (0..field_count)
        .map(|index| field_offset + index * 12)
        .find(|offset| u32_at(&bytes, *offset) == 14)
        .expect("struct field");
    let mut bad = bytes.clone();
    put_u32(&mut bad, reference_offset + 8, 999);
    assert!(matches!(
        Gff::parse("dangling", &bad),
        Err(Error::Malformed { .. })
    ));

    let mut two_fields = empty_block();
    two_fields.add_field(0, "A", Value::Byte(1)).expect("A");
    two_fields.add_field(0, "B", Value::Byte(2)).expect("B");
    let bytes = two_fields.to_bytes().expect("serialize");
    let field_indices_offset = u32_at(&bytes, 40) as usize;
    let mut bad = bytes;
    let first = u32_at(&bad, field_indices_offset);
    put_u32(&mut bad, field_indices_offset + 4, first);
    assert!(matches!(
        Gff::parse("duplicate", &bad),
        Err(Error::Malformed { .. })
    ));
}

#[test]
fn a_non_canonical_struct_offset_is_rejected() {
    let bytes = single_field("A", Value::Byte(1))
        .to_bytes()
        .expect("serialize");
    let mut bad = bytes;
    put_u32(&mut bad, 8, 60);
    assert!(matches!(
        Gff::parse("layout", &bad),
        Err(Error::Malformed { .. })
    ));
}

#[test]
fn an_empty_block_is_rejected_and_root_is_not_a_panicking_accessor() {
    let mut header = vec![0_u8; 56];
    header[..4].copy_from_slice(b"SAV ");
    header[4..8].copy_from_slice(b"V3.2");
    for index in 0..12 {
        put_u32(
            &mut header,
            8 + index * 4,
            if index % 2 == 0 { 56 } else { 0 },
        );
    }
    assert!(matches!(
        Gff::parse("empty", &header),
        Err(Error::Malformed { .. })
    ));
    assert!(empty_block().root().is_some());
}

#[test]
fn remove_rejects_an_indexed_leaf() {
    let mut gff = empty_block();
    let child = gff.push_struct(0);
    gff.add_field(child, "Value", Value::Byte(1))
        .expect("value");
    gff.add_field(0, "Child", Value::Struct(child))
        .expect("child");
    assert!(matches!(
        gff.remove("Child[0]"),
        Err(Error::FieldNotFound { .. })
    ));
    assert!(gff.get("Child").is_ok());
}

#[test]
fn the_writer_rejects_unrepresentable_values_and_versions() {
    let mut long_resref = empty_block();
    long_resref
        .add_field(0, "Long", Value::ResRef("r".repeat(300)))
        .expect("field");
    assert!(long_resref.to_bytes().is_err());

    let mut bad_version = empty_block();
    bad_version.version = *b"JUNK";
    assert!(bad_version.to_bytes().is_err());

    let mut duplicate_labels = empty_block();
    duplicate_labels.labels.push("Same".to_owned());
    duplicate_labels.labels.push("Same".to_owned());
    duplicate_labels.structs[0].fields.push(Field {
        label: 0,
        type_id: 0,
        value: Value::Byte(1),
    });
    assert!(duplicate_labels.validate("labels").is_err());
}

#[test]
fn selected_fields_still_validates_list_targets() {
    let mut gff = empty_block();
    let child = gff.push_struct(0);
    gff.add_field(child, "Value", Value::Byte(1))
        .expect("value");
    gff.add_field(0, "Items", Value::List(vec![child]))
        .expect("items");
    let bytes = gff.to_bytes().expect("serialize");
    let list_offset = u32_at(&bytes, 48) as usize;
    let mut bad = bytes;
    put_u32(&mut bad, list_offset + 4, 999);
    assert!(Gff::selected_fields("selected", &bad, &["Items"]).is_err());
}

#[test]
fn a_block_that_decodes_into_too_much_data_is_rejected() {
    let mut gff = empty_block();
    for index in 0..100 {
        gff.add_field(
            0,
            &format!("Blob{index}"),
            Value::Void(vec![0_u8; 64 * 1024]),
        )
        .expect("blob");
    }
    let bytes = gff.to_bytes().expect("serialize");
    assert!(matches!(
        Gff::parse("budget", &bytes),
        Err(Error::Malformed { .. })
    ));
}
