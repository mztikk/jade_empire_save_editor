#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Public `Value` contract: type tags, accessors and field-data classification.

use jadeforge_gff::{LocString, Value};

#[test]
fn every_variant_reports_its_name_and_type_tag() {
    let cases = [
        (Value::Byte(7), "byte", 0, false),
        (Value::Char(-5), "char", 1, false),
        (Value::Word(7), "word", 2, false),
        (Value::Short(-5), "short", 3, false),
        (Value::DWord(7), "dword", 4, false),
        (Value::Int(-5), "int", 5, false),
        (Value::DWord64(7), "dword64", 6, true),
        (Value::Int64(-5), "int64", 7, true),
        (Value::Float(1.5), "float", 8, false),
        (Value::Double(2.5), "double", 9, true),
        (Value::String("x".to_owned()), "string", 10, true),
        (Value::ResRef("x".to_owned()), "resref", 11, true),
        (
            Value::LocString(LocString::from_text("x".to_owned())),
            "locstring",
            18,
            true,
        ),
        (Value::Void(vec![1]), "void", 13, true),
        (Value::Struct(2), "struct", 14, false),
        (Value::List(vec![1]), "list", 15, false),
        (Value::Quaternion([0.0; 4]), "quaternion", 16, true),
        (Value::Vector3([0.0; 3]), "vector3", 17, true),
    ];

    for (value, name, type_id, in_field_data) in cases {
        assert_eq!(value.type_name(), name);
        assert_eq!(value.type_id(), type_id, "{name}");
        assert_eq!(value.is_in_field_data(), in_field_data, "{name}");
        if in_field_data {
            assert!(value.encode().is_some(), "{name}");
            assert!(value.encode_inline().is_none(), "{name}");
        } else {
            assert!(value.encode().is_none(), "{name}");
            assert_eq!(
                value.encode_inline().is_some(),
                !matches!(value, Value::List(_)),
                "{name}"
            );
        }
    }
}

#[test]
fn numeric_accessors_cover_signed_and_unsigned_variants() {
    assert_eq!(Value::Byte(7).as_u64(), Some(7));
    assert_eq!(Value::Char(-5).as_u64(), None);
    assert_eq!(Value::Word(7).as_u64(), Some(7));
    assert_eq!(Value::Short(-5).as_u64(), None);
    assert_eq!(Value::DWord(7).as_u64(), Some(7));
    assert_eq!(Value::Int(-5).as_u64(), None);
    assert_eq!(Value::DWord64(7).as_u64(), Some(7));
    assert_eq!(Value::DWord64(u64::MAX).as_u64(), Some(u64::MAX));
    assert_eq!(Value::Int64(-5).as_u64(), None);
    assert_eq!(Value::Float(1.5).as_u64(), None);

    assert_eq!(Value::Byte(7).as_i64(), Some(7));
    assert_eq!(Value::Char(-5).as_i64(), Some(-5));
    assert_eq!(Value::Word(7).as_i64(), Some(7));
    assert_eq!(Value::Short(-5).as_i64(), Some(-5));
    assert_eq!(Value::DWord(7).as_i64(), Some(7));
    assert_eq!(Value::Int(-5).as_i64(), Some(-5));
    assert_eq!(Value::DWord64(7).as_i64(), Some(7));
    assert_eq!(Value::DWord64(u64::MAX).as_i64(), None);
    assert_eq!(Value::Int64(-5).as_i64(), Some(-5));
    assert_eq!(Value::String("x".to_owned()).as_i64(), None);

    assert_eq!(Value::Float(1.5).as_f32(), Some(1.5));
    assert_eq!(Value::Double(2.5).as_f32(), Some(2.5));
    assert_eq!(Value::Int(3).as_f32(), Some(3.0));
    assert_eq!(Value::String("x".to_owned()).as_f32(), None);
}

#[test]
fn text_struct_and_list_accessors_cover_their_none_arms() {
    assert_eq!(Value::String("x".to_owned()).as_str(), Some("x"));
    assert_eq!(Value::ResRef("r".to_owned()).as_str(), Some("r"));
    assert_eq!(
        Value::LocString(LocString::from_text("text".to_owned())).as_str(),
        Some("text")
    );
    assert_eq!(Value::LocString(LocString::from_strref(1)).as_str(), None);
    assert_eq!(Value::Byte(1).as_str(), None);

    assert_eq!(Value::Struct(2).as_struct(), Some(2));
    assert_eq!(Value::Byte(1).as_struct(), None);

    assert_eq!(Value::List(vec![1, 2]).as_list(), Some(&[1, 2][..]));
    assert_eq!(Value::Byte(1).as_list(), None);

    let mut list = Value::List(vec![1]);
    list.as_list_mut().expect("list").push(2);
    assert_eq!(list.as_list(), Some(&[1, 2][..]));
    let mut other = Value::Byte(1);
    assert!(other.as_list_mut().is_none());
}

#[test]
fn equality_does_not_confuse_different_variants() {
    assert_ne!(Value::Byte(1), Value::Word(1));
    assert_ne!(
        Value::LocString(LocString::from_strref(1)),
        Value::String("1".to_owned())
    );
    assert_ne!(Value::Struct(1), Value::Int(1));
}
