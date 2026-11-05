#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! GFF tree helpers, mutation errors and malformed-structure decoding.

use jadeforge_gff::{Error, Field, FieldPath, Gff, Value};

fn empty() -> Gff {
    Gff {
        version: *b"V3.2",
        labels: Vec::new(),
        structs: Vec::new(),
        root: 0,
    }
}

fn root_gff() -> Gff {
    let mut gff = empty();
    let _ = gff.push_struct(u32::MAX);
    gff
}

fn tree() -> (Gff, usize, usize, usize) {
    let mut gff = empty();
    let root = gff.push_struct(u32::MAX);
    let child = gff.push_struct(0);
    let orphan = gff.push_struct(0);
    gff.add_field(root, "Child", Value::Struct(child))
        .expect("child");
    gff.add_field(root, "Other", Value::Byte(1)).expect("other");
    gff.add_field(child, "Value", Value::DWord(7))
        .expect("value");
    (gff, root, child, orphan)
}

fn simple() -> Gff {
    let mut gff = root_gff();
    gff.add_field(0, "A", Value::Byte(1)).expect("A");
    gff.add_field(0, "B", Value::Word(2)).expect("B");
    gff
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("4 bytes"))
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn tree_helpers_find_and_report_reachability() {
    let (mut gff, root, child, orphan) = tree();
    assert_eq!(
        gff.find(root, "Child").expect("child").value,
        Value::Struct(child)
    );
    assert_eq!(
        gff.find(child, "Value").expect("value").value,
        Value::DWord(7)
    );
    assert!(gff.find(root, "Missing").is_none());
    assert!(gff.find(99, "Value").is_none());

    gff.find_mut(child, "Value").expect("value").value = Value::DWord(9);
    assert_eq!(
        gff.find(child, "Value").expect("value").value,
        Value::DWord(9)
    );

    let reachable = gff.reachable();
    assert_eq!(reachable.len(), 3);
    assert!(reachable[root]);
    assert!(reachable[child]);
    assert!(!reachable[orphan]);
}

#[test]
fn path_mutations_report_missing_fields_and_parents() {
    let (mut gff, _, child, _) = tree();

    gff.add("Child.Extra", Value::Byte(3)).expect("add");
    assert_eq!(gff.get("Child.Extra").expect("field").value, Value::Byte(3));
    assert!(matches!(
        gff.add("Missing.Extra", Value::Byte(3)),
        Err(Error::FieldNotFound { .. })
    ));
    assert!(matches!(
        gff.add("Child[0]", Value::Byte(3)),
        Err(Error::FieldNotFound { .. })
    ));
    assert!(matches!(
        gff.add("Child.Value[0]", Value::Byte(3)),
        Err(Error::FieldNotFound { .. })
    ));

    let removed = gff.remove("Child.Extra").expect("remove");
    assert_eq!(removed.value, Value::Byte(3));
    assert!(matches!(
        gff.remove("Missing.Extra"),
        Err(Error::FieldNotFound { .. })
    ));
    assert!(matches!(
        gff.get_mut("Missing"),
        Err(Error::FieldNotFound { .. })
    ));
    assert!(matches!(
        gff.set("Missing", Value::Byte(1)),
        Err(Error::FieldNotFound { .. })
    ));
    assert!(matches!(
        gff.add_field(99, "New", Value::Byte(1)),
        Err(Error::Corrupt(_))
    ));
    assert!(matches!(
        gff.remove_field(99, "Value"),
        Err(Error::FieldNotFound { .. })
    ));
    assert!(matches!(
        gff.remove_field(child, "Missing"),
        Err(Error::FieldNotFound { .. })
    ));
    gff.remove_field(child, "Value").expect("remove field");
    assert!(gff.find(child, "Value").is_none());

    gff.add("Top", Value::Byte(4)).expect("root add");
    gff.remove("Top").expect("root remove");
    let ghost = gff.intern_label("Ghost").expect("ghost label");
    assert!(matches!(
        gff.remove_field(0, "Ghost"),
        Err(Error::FieldNotFound { .. })
    ));
    let _ = ghost;
    assert!(matches!(
        gff.intern_label(&"x".repeat(17)),
        Err(Error::LabelTooLong(_))
    ));
    assert!(gff.get("Child.Value[0]").is_err());
    assert!(gff.get("Children.Child").is_err());
    assert!(gff.add("Children.Child", Value::Byte(1)).is_err());
}

#[test]
fn add_field_rejects_duplicate_labels() {
    let mut gff = root_gff();
    gff.add_field(0, "Same", Value::Byte(1))
        .expect("first field");
    assert!(matches!(
        gff.add_field(0, "Same", Value::Byte(2)),
        Err(Error::Corrupt(_))
    ));
}

#[test]
fn validate_reports_duplicate_labels_and_bad_references() {
    let mut duplicate = root_gff();
    let label = duplicate.intern_label("Dup").expect("label");
    for value in [1, 2] {
        duplicate.structs[0].fields.push(Field {
            label,
            type_id: 0,
            value: Value::Byte(value),
        });
    }
    assert!(
        duplicate
            .validate("duplicate")
            .expect_err("duplicate")
            .to_string()
            .contains("twice")
    );

    let mut missing_label = root_gff();
    missing_label.structs[0].fields.push(Field {
        label: 99,
        type_id: 0,
        value: Value::Byte(1),
    });
    assert!(missing_label.validate("label").is_err());

    let mut dangling_struct = root_gff();
    dangling_struct.labels.push("Child".to_owned());
    dangling_struct.structs[0].fields.push(Field {
        label: 0,
        type_id: 14,
        value: Value::Struct(99),
    });
    assert!(dangling_struct.validate("struct").is_err());

    let mut dangling_list = root_gff();
    dangling_list.labels.push("Items".to_owned());
    dangling_list.structs[0].fields.push(Field {
        label: 0,
        type_id: 15,
        value: Value::List(vec![99]),
    });
    assert!(dangling_list.validate("list").is_err());

    let mut long_label = root_gff();
    long_label.labels.push("ThisLabelIsWayTooLong".to_owned());
    long_label.structs[0].fields.push(Field {
        label: 0,
        type_id: 0,
        value: Value::Byte(1),
    });
    assert!(long_label.validate("label").is_err());

    let mut bad_root = root_gff();
    bad_root.root = 99;
    assert!(bad_root.validate("root").is_err());

    let mut bad_field_type = root_gff();
    bad_field_type.labels.push("Mismatch".to_owned());
    bad_field_type.structs[0].fields.push(Field {
        label: 0,
        type_id: 4,
        value: Value::Byte(1),
    });
    assert!(bad_field_type.validate("type").is_err());
}

#[test]
fn parse_rejects_structural_damage() {
    let bytes = simple().to_bytes().expect("serialize");
    let struct_offset = u32_at(&bytes, 8) as usize;
    let field_offset = u32_at(&bytes, 16) as usize;
    let label_offset = u32_at(&bytes, 24) as usize;
    let field_indices_offset = u32_at(&bytes, 40) as usize;

    let mut block_size = bytes.clone();
    block_size.push(0);
    assert!(matches!(
        Gff::parse("block-size", &block_size),
        Err(Error::Malformed { .. })
    ));

    let mut unaligned = bytes.clone();
    put_u32(&mut unaligned, struct_offset + 4, 2);
    assert!(matches!(
        Gff::parse("unaligned", &unaligned),
        Err(Error::Malformed { .. })
    ));

    let mut missing_field = bytes.clone();
    put_u32(&mut missing_field, field_indices_offset, 99);
    assert!(matches!(
        Gff::parse("field", &missing_field),
        Err(Error::Malformed { .. })
    ));

    let mut bad_label = bytes.clone();
    bad_label[label_offset] = 0xff;
    assert!(matches!(
        Gff::parse("label", &bad_label),
        Err(Error::Malformed { .. })
    ));

    let mut section = bytes.clone();
    put_u32(&mut section, 36, u32::MAX);
    assert!(matches!(
        Gff::parse("section", &section),
        Err(Error::Malformed { .. })
    ));

    let mut bad_label_index = bytes;
    put_u32(&mut bad_label_index, field_offset + 4, 99);
    assert!(matches!(
        Gff::parse("label-index", &bad_label_index),
        Err(Error::Malformed { .. })
    ));
}

#[test]
fn selected_fields_reads_paths_and_rejects_damage() {
    let gff = simple();
    let bytes = gff.to_bytes().expect("serialize");
    let selected =
        Gff::selected_fields("selected", &bytes, &["A", "B", "A.None", ""]).expect("selected");
    assert_eq!(selected[0], Some(Value::Byte(1)));
    assert_eq!(selected[1], Some(Value::Word(2)));
    assert_eq!(selected[2], None);
    assert_eq!(selected[3], None);

    // A path through a scalar field stops at that field.
    let selected = Gff::selected_fields("scalar", &bytes, &["A.Child"]).expect("selected");
    assert_eq!(selected[0], None);

    let struct_offset = u32_at(&bytes, 8) as usize;
    let field_offset = u32_at(&bytes, 16) as usize;
    let label_offset = u32_at(&bytes, 24) as usize;
    let field_indices_offset = u32_at(&bytes, 40) as usize;

    let mut bad_magic = bytes.clone();
    bad_magic[..4].copy_from_slice(b"NOPE");
    assert!(Gff::selected_fields("magic", &bad_magic, &["A"]).is_err());

    let mut bad_version = bytes.clone();
    bad_version[4..8].copy_from_slice(b"V2.0");
    assert!(Gff::selected_fields("version", &bad_version, &["A"]).is_err());

    let mut block_size = bytes.clone();
    block_size.push(0);
    assert!(Gff::selected_fields("block-size", &block_size, &["A"]).is_err());

    let mut section = bytes.clone();
    put_u32(&mut section, 36, u32::MAX);
    assert!(Gff::selected_fields("section", &section, &["A"]).is_err());

    let mut unaligned = bytes.clone();
    put_u32(&mut unaligned, struct_offset + 4, 2);
    assert!(Gff::selected_fields("unaligned", &unaligned, &["A"]).is_err());

    let mut missing_field = bytes.clone();
    put_u32(&mut missing_field, field_indices_offset, 99);
    assert!(Gff::selected_fields("field", &missing_field, &["A"]).is_err());

    let mut bad_label = bytes.clone();
    bad_label[label_offset] = 0xff;
    assert!(Gff::selected_fields("label", &bad_label, &["A"]).is_err());

    let mut bad_label_index = bytes;
    put_u32(&mut bad_label_index, field_offset + 4, 99);
    assert!(Gff::selected_fields("label-index", &bad_label_index, &["A"]).is_err());
}

#[test]
fn field_path_parent_and_display_are_consistent() {
    assert!(FieldPath::parse("Gold").expect("path").parent().is_none());
    let path = FieldPath::parse("Stats.Focus.STAT_CUR").expect("path");
    assert_eq!(path.parent().expect("parent").to_string(), "Stats.Focus");
    assert_eq!(
        FieldPath::parse("ItemList[2].Count")
            .expect("path")
            .to_string(),
        "ItemList[2].Count"
    );
}
