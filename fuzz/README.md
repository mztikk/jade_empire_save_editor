# Fuzz targets

`cargo-fuzz` targets for the three untrusted-input parsers. They are not part of
the workspace build (the root `Cargo.toml` excludes `fuzz/`), because they need
nightly Rust and libFuzzer. The deterministic mutation tests in
`crates/jadeforge-gff/tests/mutations.rs` and
`crates/jadeforge-gamedata/tests/mutations.rs` run on every CI job; the fuzzers
are for longer local or nightly runs.

```sh
cargo install cargo-fuzz
cargo +nightly fuzz run gff -- -max_total_time=300
cargo +nightly fuzz run save -- -max_total_time=300
cargo +nightly fuzz run twoda -- -max_total_time=300
```

A crash artifact belongs in the corresponding regression test before it is
fixed. The reader enforces a decoded-data budget, bounded allocation and strict
layout checks, so timeout-only findings should be investigated as an algorithmic
complexity bug rather than ignored.
