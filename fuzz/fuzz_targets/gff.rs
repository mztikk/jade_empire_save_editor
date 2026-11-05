#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = jadeforge_gff::Gff::parse("fuzz", data);
});
