#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Field codec and malformed-block tests.
//!
//! The savegame tests cover the field types that real savegames use. This test
//! builds a block containing *every* field type and mutates the serialized
//! bytes to reach the parser's error paths.

use jadeforge_gff::{Error, Field, Gff, LocString, Struct, VERSION, Value};

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
fn selected_fields_decode_only_the_requested_paths() {
    let mut gff = empty_block();
    let child = gff.push_struct(0);
    gff.add_field(child, "Number", Value::DWord(7))
        .expect("child field");
    gff.add_field(0, "Child", Value::Struct(child))
        .expect("struct field");
    gff.add_field(0, "Other", Value::Byte(1))
        .expect("other field");
    let bytes = gff.to_bytes().expect("serialize");
    let selected = Gff::selected_fields(
        "selected",
        &bytes,
        &["Child.Number", "Child.Missing", "Other"],
    )
    .expect("selected fields");
    assert_eq!(selected[0], Some(Value::DWord(7)));
    assert_eq!(selected[1], None);
    assert_eq!(selected[2], Some(Value::Byte(1)));

    let mut with_list = empty_block();
    let element = with_list.push_struct(0);
    with_list
        .add_field(element, "Element", Value::Byte(9))
        .expect("element");
    with_list
        .add_field(0, "Items", Value::List(vec![element]))
        .expect("list");
    let bytes = with_list.to_bytes().expect("serialize");
    let selected = Gff::selected_fields("selected", &bytes, &["Items"]).expect("list");
    assert_eq!(selected[0], Some(Value::List(vec![element])));
}

#[test]
fn selected_fields_reject_malformed_blocks() {
    assert!(Gff::selected_fields("empty", &[], &["A"]).is_err());
    assert!(Gff::selected_fields("magic", b"NOPE", &["A"]).is_err());
    let bytes = single_field("A", Value::Byte(1))
        .to_bytes()
        .expect("serialize");
    let mut truncated = bytes.clone();
    truncated.truncate(8);
    assert!(Gff::selected_fields("truncated", &truncated, &["A"]).is_err());
}

#[test]
fn every_field_type_round_trips() {
    let mut gff = empty_block();
    let child = gff.push_struct(0);
    gff.add_field(child, "ChildWord", Value::Word(7))
        .expect("child field");
    let second = gff.push_struct(0);
    gff.add_field(second, "SecondDWord", Value::DWord(11))
        .expect("second field");

    let fields = [
        ("ByteField", Value::Byte(255)),
        ("CharField", Value::Char(-5)),
        ("WordField", Value::Word(65_535)),
        ("ShortField", Value::Short(-300)),
        ("DWordField", Value::DWord(4_000_000_000)),
        ("IntField", Value::Int(-2_000_000)),
        ("DWord64Field", Value::DWord64(u64::MAX - 1)),
        ("Int64Field", Value::Int64(i64::MIN + 1)),
        ("FloatField", Value::Float(1.5)),
        ("DoubleField", Value::Double(-2.25)),
        ("StringField", Value::String("hello world".to_owned())),
        ("ResRefField", Value::ResRef("item_gem_body_I".to_owned())),
        ("LocStringRef", Value::LocString(LocString::from_strref(42))),
        (
            "LocStringInline",
            Value::LocString(LocString {
                strref: Some(7),
                text: Some("inline text".to_owned()),
            }),
        ),
        ("VoidField", Value::Void(vec![1, 2, 3, 4, 5])),
        ("StructField", Value::Struct(child)),
        ("ListField", Value::List(vec![child, second])),
        ("QuaternionField", Value::Quaternion([0.5, -0.5, 1.0, -1.0])),
        ("VectorField", Value::Vector3([1.0, 2.0, 3.0])),
    ];
    for (label, value) in fields {
        gff.add_field(0, label, value).expect("root field");
    }

    let bytes = gff.to_bytes().expect("serialize");
    let parsed = Gff::parse("codec", &bytes).expect("parse");
    assert_eq!(parsed, gff, "the round trip changed the tree");
    assert_eq!(parsed.to_bytes().expect("re-serialize"), bytes);

    // Accessors, so the never-called helper arms are exercised too.
    assert_eq!(Value::Byte(3).as_u64(), Some(3));
    assert_eq!(Value::Char(-3).as_u64(), Some((-3_i8) as u64));
    assert_eq!(Value::Short(-3).as_u64(), Some((-3_i16) as u64));
    assert_eq!(Value::Int(-3).as_i64(), Some(-3));
    assert_eq!(Value::DWord64(9).as_u64(), Some(9));
    assert_eq!(Value::Double(1.5).as_f32(), Some(1.5));
    assert_eq!(Value::Float(1.5).as_f32(), Some(1.5));
    assert_eq!(Value::String("x".to_owned()).as_str(), Some("x"));
    assert_eq!(Value::LocString(LocString::from_strref(1)).as_str(), None);
    assert_eq!(Value::Struct(2).as_struct(), Some(2));
    assert_eq!(Value::List(vec![1, 2]).as_list(), Some(&[1, 2][..]));
    let mut list = Value::List(vec![1]);
    list.as_list_mut().expect("list").push(2);
    assert_eq!(list.as_list(), Some(&[1, 2][..]));
    assert!(Value::Byte(1).as_list_mut().is_none());
}

#[test]
fn empty_and_maximal_payloads_round_trip() {
    let mut gff = empty_block();
    let long_resref = "r".repeat(255);
    for (label, value) in [
        ("EmptyString", Value::String(String::new())),
        ("EmptyResRef", Value::ResRef(String::new())),
        ("LongResRef", Value::ResRef(long_resref)),
        ("EmptyVoid", Value::Void(Vec::new())),
        (
            "EmptyLoc",
            Value::LocString(LocString {
                strref: None,
                text: None,
            }),
        ),
        ("ZeroStrref", Value::LocString(LocString::from_strref(0))),
        (
            "TextOnly",
            Value::LocString(LocString {
                strref: None,
                text: Some("text only".to_owned()),
            }),
        ),
    ] {
        gff.add_field(0, label, value).expect("edge field");
    }
    let bytes = gff.to_bytes().expect("serialize");
    let parsed = Gff::parse("edges", &bytes).expect("parse");
    assert_eq!(parsed, gff, "an edge payload changed in the round trip");
}

#[test]
fn structural_edge_cases_round_trip() {
    let mut gff = empty_block();
    let child = gff.push_struct(0);
    gff.add_field(child, "Only", Value::Byte(1))
        .expect("single field child");
    gff.add_field(0, "EmptyList", Value::List(Vec::new()))
        .expect("empty list");
    gff.add_field(0, "Repeated", Value::List(vec![child, child]))
        .expect("repeated reference");
    gff.add_field(0, "Single", Value::Struct(child))
        .expect("struct");
    // Exactly sixteen bytes: the label is not NUL terminated on disk.
    gff.add_field(0, "ABCDEFGHIJKLMNOP", Value::DWord(3))
        .expect("max label");
    let bytes = gff.to_bytes().expect("serialize");
    let parsed = Gff::parse("structure", &bytes).expect("parse");
    assert_eq!(parsed, gff);
    // A one-field struct is stored inline; the parser must handle that.
    assert_eq!(parsed.struct_at(child).expect("child").fields.len(), 1);
}

#[test]
fn legacy_locstring_type_is_read() {
    let gff = single_field(
        "Text",
        Value::LocString(LocString {
            strref: None,
            text: Some("legacy".to_owned()),
        }),
    );
    let mut bytes = gff.to_bytes().expect("serialize");
    let field_offset = u32_at(&bytes, 16) as usize;
    // Type 12 is the legacy locstring tag; the decoder treats it like type 18.
    put_u32(&mut bytes, field_offset, 12);
    let parsed = Gff::parse("legacy", &bytes).expect("parse legacy locstring");
    assert_eq!(
        parsed.struct_at(0).expect("root").fields[0].value,
        gff.root().fields[0].value
    );
}

#[test]
fn malformed_blocks_are_rejected() {
    let gff = single_field("Value", Value::Word(1));
    let bytes = gff.to_bytes().expect("serialize");

    // Header level errors.
    assert!(matches!(
        Gff::parse("short", &bytes[..40]),
        Err(Error::Malformed { .. })
    ));
    let mut bad = bytes.clone();
    bad[..4].copy_from_slice(b"BAD ");
    assert!(matches!(
        Gff::parse("magic", &bad),
        Err(Error::Malformed { .. })
    ));
    let mut bad = bytes.clone();
    bad[4..8].copy_from_slice(b"V2.0");
    assert!(matches!(
        Gff::parse("version", &bad),
        Err(Error::Malformed { .. })
    ));

    let field_offset = u32_at(&bytes, 16) as usize;

    // Label index out of bounds.
    let mut bad = bytes.clone();
    put_u32(&mut bad, field_offset + 4, 999);
    assert!(matches!(
        Gff::parse("label", &bad),
        Err(Error::Malformed { .. })
    ));

    // Unknown inline type.
    let mut bad = bytes.clone();
    put_u32(&mut bad, field_offset, 99);
    assert!(matches!(
        Gff::parse("type", &bad),
        Err(Error::Malformed { .. })
    ));

    // List element out of bounds.
    let gff = single_field("Items", Value::List(vec![0]));
    let bytes = gff.to_bytes().expect("serialize");
    let list_offset = u32_at(&bytes, 48) as usize;
    let mut bad = bytes.clone();
    put_u32(&mut bad, list_offset + 4, 999);
    assert!(matches!(
        Gff::parse("list", &bad),
        Err(Error::Malformed { .. })
    ));

    // String length beyond the field data.
    let gff = single_field("Text", Value::String("abc".to_owned()));
    let bytes = gff.to_bytes().expect("serialize");
    let data_offset = u32_at(&bytes, 32) as usize;
    let mut bad = bytes.clone();
    put_u32(&mut bad, data_offset, u32::MAX);
    assert!(matches!(
        Gff::parse("string", &bad),
        Err(Error::Malformed { .. })
    ));

    // Truncated locstring.
    let gff = single_field(
        "Loc",
        Value::LocString(LocString {
            strref: None,
            text: Some("x".to_owned()),
        }),
    );
    let bytes = gff.to_bytes().expect("serialize");
    let data_offset = u32_at(&bytes, 32) as usize;
    let mut bad = bytes.clone();
    put_u32(&mut bad, data_offset, 1);
    assert!(matches!(
        Gff::parse("locstring", &bad),
        Err(Error::Malformed { .. })
    ));
}

#[test]
fn invalid_trees_are_rejected_before_writing() {
    assert!(
        Gff {
            version: VERSION,
            labels: Vec::new(),
            structs: Vec::new(),
            root: 0,
        }
        .to_bytes()
        .is_err(),
        "a block without structs must fail"
    );

    let mut root_out_of_bounds = empty_block();
    root_out_of_bounds.root = 3;
    assert!(root_out_of_bounds.to_bytes().is_err());

    let mut long_label = empty_block();
    let index = u32::try_from(long_label.labels.len()).expect("label index");
    long_label
        .labels
        .push("ThisLabelIsWayTooLongForTheFormat".to_owned());
    long_label.structs[0].fields.push(Field {
        label: index,
        type_id: 0,
        value: Value::Byte(1),
    });
    assert!(long_label.to_bytes().is_err());

    let mut duplicate = empty_block();
    duplicate
        .add_field(0, "Dup", Value::Byte(1))
        .expect("first field");
    duplicate
        .add_field(0, "Dup", Value::Byte(2))
        .expect_err("duplicate labels are rejected");

    let mut wrong_type = empty_block();
    let label = wrong_type.intern_label("Mismatch").expect("intern");
    wrong_type.structs[0].fields.push(Field {
        label,
        type_id: 4,
        value: Value::Byte(1),
    });
    assert!(wrong_type.to_bytes().is_err());

    let mut dangling = empty_block();
    dangling
        .add_field(0, "Dangling", Value::Struct(99))
        .expect("field");
    assert!(dangling.to_bytes().is_err());
}
