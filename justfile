# JadeForge task runner. `just` lists the recipes and `just check` runs the
# quality gate from AGENTS.md. The plain cargo commands keep working.
#
# Recipes run through bash with `-e -o pipefail` (Git Bash on Windows). Tests
# that need real savegames read JADEFORGE_SAVE_DIR, e.g.
# `JADEFORGE_SAVE_DIR="$HOME/games/Jade Empire/Save" just test`.
set shell := ["bash", "-e", "-o", "pipefail", "-c"]

# Workspace version and the triple rustc builds for. They name the
# distribution folders and pick the native target.
version := trim(shell("awk -F'\"' '/^version = / { print $2; exit }' Cargo.toml"))
host := trim(shell("rustc -vV | sed -n 's/^host: //p'"))
exe := if os() == "windows" { ".exe" } else { "" }
target_system := if os() == "macos" { "darwin" } else { os() }

# List the recipes.
default:
    @just --list

# Build the whole workspace in debug mode.
build:
    cargo build --workspace

# Run every test with nextest, plus doctests (nextest does not run those).
# Tests that need savegames run when JADEFORGE_SAVE_DIR is set.
test:
    cargo nextest run --workspace
    cargo test --workspace --doc

# Lint the whole workspace and deny warnings.
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Format the workspace.
fmt:
    cargo fmt --all

# Fail when the workspace is not formatted.
fmt-check:
    cargo fmt --all --check

# The quality gate: formatting, lints and tests.
check: fmt-check clippy test

# Report the lints that are not gated yet (needs jq).
strict:
    cargo clippy --workspace --all-targets --message-format=json -- \
        -W clippy::expect_used \
        -W clippy::indexing_slicing \
        -W clippy::arithmetic_side_effects \
        -W clippy::nursery 2>/dev/null \
    | jq -r 'select(.reason == "compiler-message") | .message | select(.level == "warning" or .level == "error") | .code.code // "unknown"' \
    | sort | uniq -c | sort -rn \
    | awk '{ line[NR] = sprintf("%5d  %s", $1, $2); total += $1 } END { print total " strict diagnostics"; for (i = 1; i <= NR; i++) print line[i] }'

# Code coverage: line/function summary, coverage/lcov.info and coverage/html.
# Needs cargo-llvm-cov and llvm-tools-preview (or LLVM_COV/LLVM_PROFDATA).
coverage *args:
    mkdir -p coverage
    cargo llvm-cov nextest --workspace --summary-only \
        --lcov --output-path coverage/lcov.info {{ args }}
    cargo llvm-cov report --html --output-dir coverage

# Render the UI into PNG files without opening a window.
snapshot save_dir out="docs/screenshot.png" *args:
    cargo run -q -p jadeforge-gui --example snapshot -- "{{ save_dir }}" "{{ out }}" {{ args }}

# Start the desktop application.
run:
    cargo run -p jadeforge-gui

# Run the command line tool, e.g. `just cli info "<SAVE>"`.
cli *args:
    cargo run -q -p jadeforge-cli -- {{ args }}

# Build and stage the fat-LTO binaries for one target triple.
#
# x86-64 Linux and Windows build twice, into the same folder: `jadeforge` and
# `jadeforge-cli` with the x86-64-v3 ISA (AVX2, Haswell/Zen 2+), plus
# `jadeforge-baseline` and `jadeforge-cli-baseline` for older CPUs. macOS
# builds once, because the Intel binary must also run under Rosetta (no AVX2).
publish-target target:
    @case "{{ target }}" in \
        *-{{ target_system }}*) ;; \
        *) echo "publish: {{ target }} is not buildable on {{ os() }}"; exit 1 ;; \
    esac
    @if command -v rustup >/dev/null 2>&1 \
        && ! rustup target list --installed 2>/dev/null | grep -Fxq '{{ target }}'; then \
        rustup target add {{ target }}; \
    fi
    dual=0; case "{{ target }}" in x86_64-unknown-linux-gnu|x86_64-pc-windows-msvc) dual=1 ;; esac; \
    if [ "$dual" = 1 ]; then \
        RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --locked --profile dist --target {{ target }} -p jadeforge-gui -p jadeforge-cli; \
        CARGO_TARGET_DIR="target/baseline" cargo build --locked --profile dist --target {{ target }} -p jadeforge-gui -p jadeforge-cli; \
    else \
        cargo build --locked --profile dist --target {{ target }} -p jadeforge-gui -p jadeforge-cli; \
    fi
    dual=0; case "{{ target }}" in x86_64-unknown-linux-gnu|x86_64-pc-windows-msvc) dual=1 ;; esac; \
    out="dist/jadeforge-{{ version }}-{{ target }}"; \
    mkdir -p "$out"; \
    rm -f "$out/jadeforge{{ exe }}" "$out/jadeforge-baseline{{ exe }}" \
        "$out/jadeforge-cli{{ exe }}" "$out/jadeforge-cli-baseline{{ exe }}" \
        "$out/jadeforge-gui{{ exe }}"; \
    cp "target/{{ target }}/dist/jadeforge{{ exe }}" "$out/"; \
    cp "target/{{ target }}/dist/jadeforge-cli{{ exe }}" "$out/"; \
    if [ "$dual" = 1 ]; then \
        cp "target/baseline/{{ target }}/dist/jadeforge{{ exe }}" "$out/jadeforge-baseline{{ exe }}"; \
        cp "target/baseline/{{ target }}/dist/jadeforge-cli{{ exe }}" "$out/jadeforge-cli-baseline{{ exe }}"; \
    fi; \
    cp LICENSE README.md "$out/"; \
    echo "publish: staged $out/"

# Build the fat-LTO binaries for every target this host can build.
[linux]
publish: (publish-target host)
    @echo "publish: skipped Windows: it needs a Windows host (Visual C++ for Skia)"
    @echo "publish: skipped macOS: it needs the Apple SDK on a macOS host"

# Build the fat-LTO binaries for every target this host can build.
[macos]
publish: (publish-target "aarch64-apple-darwin") (publish-target "x86_64-apple-darwin")
    @echo "publish: skipped Windows: it needs a Windows host (Visual C++ for Skia)"

# Build the fat-LTO binaries for every target this host can build.
[windows]
publish: (publish-target host)
    @echo "publish: skipped macOS: it needs the Apple SDK on a macOS host"


# Build SteamOS/Steam Deck-compatible Linux binaries with rootless podman.
# Produces the v3 binaries and the baseline variants in one folder.
publish-linux:
    @command -v podman >/dev/null 2>&1 || { \
        echo "publish-linux needs rootless podman (https://podman.io)"; exit 1; \
    }
    podman build -t jadeforge-build:22.04 -f ci/Containerfile .
    podman run --rm -v "$PWD":/src -w /src \
        -e CARGO_TARGET_DIR=/src/target/podman \
        -e RUSTFLAGS="-C target-cpu=x86-64-v3" \
        -v jadeforge-cargo-registry:/root/.cargo/registry \
        jadeforge-build:22.04 cargo build --locked --profile dist \
        -p jadeforge-gui -p jadeforge-cli
    podman run --rm -v "$PWD":/src -w /src \
        -e CARGO_TARGET_DIR=/src/target/podman-baseline \
        -e RUSTFLAGS= \
        -v jadeforge-cargo-registry:/root/.cargo/registry \
        jadeforge-build:22.04 cargo build --locked --profile dist \
        -p jadeforge-gui -p jadeforge-cli
    version=$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml); \
    host=$(rustc -vV | sed -n 's/^host: //p'); \
    out="dist/jadeforge-$version-$host"; \
    mkdir -p "$out"; \
    rm -f "$out/jadeforge" "$out/jadeforge-baseline" \
        "$out/jadeforge-cli" "$out/jadeforge-cli-baseline" \
        "$out/jadeforge-gui"; \
    cp "target/podman/dist/jadeforge" "$out/"; \
    cp "target/podman/dist/jadeforge-cli" "$out/"; \
    cp "target/podman-baseline/dist/jadeforge" "$out/jadeforge-baseline"; \
    cp "target/podman-baseline/dist/jadeforge-cli" "$out/jadeforge-cli-baseline"; \
    cp LICENSE README.md "$out/"; \
    ci/check-glibc.sh "$out/jadeforge" 2.35; \
    ci/check-glibc.sh "$out/jadeforge-baseline" 2.35; \
    echo "publish-linux: staged $out/"

# Remove all build artifacts: the cargo target directory and the staged
# distribution binaries in dist/.
clean:
    cargo clean
    rm -rf dist
