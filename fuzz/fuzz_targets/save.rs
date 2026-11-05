#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = jadeforge_core::SaveGame::parse("fuzz.sac".into(), data.to_vec());
});
