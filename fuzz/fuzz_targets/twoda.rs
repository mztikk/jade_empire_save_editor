#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = jadeforge_gamedata::twoda::TwoDa::parse_binary(data);
    let text = String::from_utf8_lossy(data);
    let _ = jadeforge_gamedata::twoda::TwoDa::parse(&text);
    let _ = jadeforge_gamedata::talktable::TalkTable::parse("fuzz.tlk".into(), data.to_vec());
});
