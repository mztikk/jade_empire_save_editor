#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! A deterministic mutation pass over the GFF reader.
//!
//! This is deliberately dependency-free and runs in CI: every mutant must
//! either parse or return a readable error. A panic, a hang or an allocation
//! abort fails the test.

use jadeforge_gff::{Field, Gff, Struct, VERSION, Value};

fn valid_block() -> Vec<u8> {
    let mut gff = Gff {
        version: VERSION,
        labels: Vec::new(),
        structs: vec![Struct {
            type_id: u32::MAX,
            fields: Vec::new(),
        }],
        root: 0,
    };
    let child = gff.push_struct(0);
    gff.add_field(child, "Value", Value::DWord(7))
        .expect("child field");
    gff.add_field(0, "Ref", Value::Struct(child))
        .expect("struct field");
    gff.add_field(0, "List", Value::List(vec![child]))
        .expect("list field");
    gff.add_field(0, "Text", Value::String("hello".to_owned()))
        .expect("text field");
    gff.add_field(0, "Bytes", Value::Void(vec![1, 2, 3, 4]))
        .expect("void field");
    gff.add_field(
        0,
        "Loc",
        Value::LocString(jadeforge_gff::LocString::from_text("world".to_owned())),
    )
    .expect("locstring");
    gff.labels.push("Extra".to_owned());
    gff.structs[0].fields.push(Field {
        label: u32::try_from(gff.labels.len() - 1).expect("label"),
        type_id: 0,
        value: Value::Byte(1),
    });
    gff.to_bytes().expect("valid block")
}

#[test]
fn random_mutations_never_panic_the_reader() {
    let original = valid_block();
    let mut state = 0x1234_5678_9abc_def0_u64;
    for _ in 0..1_000 {
        let mut bytes = original.clone();
        let mutations = 1 + (state as usize % 4);
        for _ in 0..mutations {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let index = (state as usize) % bytes.len();
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            bytes[index] = (state & 0xff) as u8;
        }
        let _ = Gff::parse("mutant", &bytes);
    }
}

#[test]
fn truncations_and_empty_input_are_readable_errors() {
    let original = valid_block();
    for length in 0..original.len().min(200) {
        let _ = Gff::parse("truncated", &original[..length]);
    }
    let _ = Gff::parse("empty", &[]);
}
