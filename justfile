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

# Run every test, including the ones that need real savegames.
test:
    cargo test --workspace

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
publish-target target:
    @case "{{ target }}" in \
        *-{{ target_system }}*) ;; \
        *) echo "publish: {{ target }} is not buildable on {{ os() }}"; exit 1 ;; \
    esac
    @if command -v rustup >/dev/null 2>&1 \
        && ! rustup target list --installed 2>/dev/null | grep -Fxq '{{ target }}'; then \
        rustup target add {{ target }}; \
    fi
    cargo build --locked --profile dist --target {{ target }} -p jadeforge-gui -p jadeforge-cli
    mkdir -p "dist/jadeforge-{{ version }}-{{ target }}"
    cp "target/{{ target }}/dist/jadeforge{{ exe }}" "dist/jadeforge-{{ version }}-{{ target }}/"
    cp "target/{{ target }}/dist/jadeforge-gui{{ exe }}" "dist/jadeforge-{{ version }}-{{ target }}/"
    cp LICENSE README.md "dist/jadeforge-{{ version }}-{{ target }}/"
    @echo "publish: staged dist/jadeforge-{{ version }}-{{ target }}/"

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

# Remove build artifacts.
clean:
    cargo clean
