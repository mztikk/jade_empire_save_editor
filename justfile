# JadeForge task runner. `just` lists the recipes and `just check` runs the
# quality gate from AGENTS.md. The plain cargo commands keep working.
#
# Every task runs on its own branch: `just setup` once per clone,
# `just start <type>/<slug>` to begin and `just finish` before handing over.
#
# Recipes run through bash with `-e -o pipefail` (Git Bash on Windows). Tests
# that need real savegames read JADEFORGE_SAVE_DIR, e.g.
# `JADEFORGE_SAVE_DIR="$HOME/games/Jade Empire/Save" just test`.
set shell := ["bash", "-e", "-o", "pipefail", "-c"]
set positional-arguments

# Workspace version and the triple rustc builds for. They name the
# distribution folders and pick the native target.
version := trim(shell("awk -F'\"' '/^version = / { print $2; exit }' Cargo.toml"))
host := trim(shell("rustc -vV | sed -n 's/^host: //p'"))
exe := if os() == "windows" { ".exe" } else { "" }
target_system := if os() == "macos" { "darwin" } else { os() }

# List the recipes.
default:
    @just --list

# Install the tracked git hooks (once per clone). The pre-commit hook refuses
# direct commits on main; see AGENTS.md "Branch workflow".
setup:
    git config --local core.hooksPath .githooks
    @printf 'setup: core.hooksPath=%s\n' "$(git config --local core.hooksPath)"

# Start a task branch from the latest origin/main. The name must look like
# <type>/<slug>; the worktree must be clean so no work follows you across.
start branch:
    ci/git-workflow.sh start {{quote(branch)}}

# Fail unless the task is finished: not on main, nothing uncommitted and at
# least one commit beyond origin/main. This is the hand-over guard, not the
# quality gate; run `just check` first.
finish:
    ci/git-workflow.sh finish

# Build the whole workspace in debug mode.
build:
    cargo build --workspace

# Run every test with nextest, plus doctests (nextest does not run those).
# Tests that need savegames run when JADEFORGE_SAVE_DIR is set.
test:
    cargo nextest run --workspace
    cargo test --workspace --doc

# NEVER run without explicit permission (AGENTS.md rule 13).
# Windowed UI tests on the real Wayland desktop; only when a headless UI test
# cannot cover the issue or the bug is in the real window. Opens real windows
# and takes focus. See docs/LIVE-UI.md.
#
# The approval variable is a second gate against accidental invocation (for
# example shell command substitution in a quoted argument): plain `just live`
# now refuses before the test binary can open a window.
live:
    if [ "${JADEFORGE_LIVE_APPROVED:-}" != "1" ]; then \
        echo "refusing the windowed UI suite: obtain explicit permission, then run JADEFORGE_LIVE_APPROVED=1 just live" >&2; \
        exit 1; \
    fi
    JADEFORGE_LIVE=1 JADEFORGE_LIVE_APPROVED=1 cargo test -p jadeforge-gui --test live -- --nocapture

# Lint the whole workspace and deny warnings.
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Fail when rustdoc would warn about broken intra-doc links or other docs.
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps

# Format the workspace.
fmt:
    cargo fmt --all

# Fail when the workspace is not formatted.
fmt-check:
    cargo fmt --all --check

# The quality gate: formatting, lints, rustdoc and tests.
check: fmt-check clippy doc-check test

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
    cargo llvm-cov nextest --workspace {{ args }}
    cargo llvm-cov report --lcov --output-path coverage/lcov.info
    cargo llvm-cov report --html --output-dir coverage

# Render the UI into PNG files without opening a window.
snapshot save_dir out="docs/screenshot.png" *args:
    shift 2; cargo run -q -p jadeforge-gui --example snapshot -- "{{ save_dir }}" "{{ out }}" "$@"

# Start the desktop application.
run:
    cargo run -p jadeforge-gui

# Run the command line tool, e.g. `just cli info "<SAVE>"`.
cli *args:
    cargo run -q -p jadeforge-cli -- "$@"

# Install the GUI and CLI into ~/.cargo/bin, tuned for this machine's CPU. Not
# portable: use `just publish` for binaries meant for other machines.
install:
    RUSTFLAGS="-C target-cpu=native" cargo install --path crates/jadeforge-gui --locked --profile dist --force
    RUSTFLAGS="-C target-cpu=native" cargo install --path crates/jadeforge-cli --locked --profile dist --force

# Regenerate the PNG, ICO and ICNS application icon assets from assets/icon.svg.
icons:
    python3 ci/icons.py

# Install the GUI and CLI, the Linux launcher entry and hicolor icons for the
# current user, so Wayland compositors can find the icon by app id. The
# launcher entry expects `jadeforge` on PATH (the location `just install` uses).
[linux]
install-desktop: install
    install -Dm644 crates/jadeforge-gui/assets/linux/jadeforge.desktop "$HOME/.local/share/applications/jadeforge.desktop"
    for size in 16 24 32 48 64 128 256 512; do \
        install -Dm644 "crates/jadeforge-gui/assets/icon-$size.png" \
            "$HOME/.local/share/icons/hicolor/${size}x${size}/apps/jadeforge.png"; \
    done
    @if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database "$HOME/.local/share/applications" || true; fi
    @if command -v gtk-update-icon-cache >/dev/null 2>&1; then gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" || true; fi

# Build and stage the fat-LTO binaries for one target triple.
#
# x86-64 Linux and Windows build twice, into the same folder: `jadeforge` and
# `jadeforge-cli` with the x86-64-v3 ISA (AVX2, Haswell/Zen 2+), plus
# `jadeforge-generic-cpu` and `jadeforge-cli-generic-cpu` for older CPUs. macOS
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
    rm -f "$out/jadeforge{{ exe }}" "$out/jadeforge-generic-cpu{{ exe }}" \
        "$out/jadeforge-cli{{ exe }}" "$out/jadeforge-cli-generic-cpu{{ exe }}" \
        "$out/jadeforge-baseline{{ exe }}" "$out/jadeforge-cli-baseline{{ exe }}" \
        "$out/jadeforge-gui{{ exe }}"; \
    cp "target/{{ target }}/dist/jadeforge{{ exe }}" "$out/"; \
    cp "target/{{ target }}/dist/jadeforge-cli{{ exe }}" "$out/"; \
    if [ "$dual" = 1 ]; then \
        cp "target/baseline/{{ target }}/dist/jadeforge{{ exe }}" "$out/jadeforge-generic-cpu{{ exe }}"; \
        cp "target/baseline/{{ target }}/dist/jadeforge-cli{{ exe }}" "$out/jadeforge-cli-generic-cpu{{ exe }}"; \
    fi; \
    cp LICENSE NOTICE README.md "$out/"; \
    case "{{ target }}" in \
        *-linux-*) ci/stage-linux-desktop.sh "$out" ;; \
        *-apple-darwin) ci/macos-bundle.sh "$out/JadeForge.app" "$out/jadeforge" "{{ version }}" ;; \
    esac; \
    ci/notices.sh "$out" >/dev/null; \
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
    rm -f "$out/jadeforge" "$out/jadeforge-generic-cpu" \
        "$out/jadeforge-cli" "$out/jadeforge-cli-generic-cpu" \
        "$out/jadeforge-baseline" "$out/jadeforge-cli-baseline" \
        "$out/jadeforge-gui"; \
    cp "target/podman/dist/jadeforge" "$out/"; \
    cp "target/podman/dist/jadeforge-cli" "$out/"; \
    cp "target/podman-baseline/dist/jadeforge" "$out/jadeforge-generic-cpu"; \
    cp "target/podman-baseline/dist/jadeforge-cli" "$out/jadeforge-cli-generic-cpu"; \
    cp LICENSE NOTICE README.md "$out/"; \
    ci/stage-linux-desktop.sh "$out"; \
    ci/notices.sh "$out" >/dev/null; \
    ci/check-glibc.sh "$out/jadeforge" 2.35; \
    ci/check-glibc.sh "$out/jadeforge-generic-cpu" 2.35; \
    echo "publish-linux: staged $out/"

# Collect the licence inventory for the staged distribution.
notices:
    ci/notices.sh dist

# Generate the CycloneDX SBOM for the staged distribution.
sbom:
    ci/sbom.sh dist

# Write SHA-256 checksums for the staged distribution.
checksums:
    cd dist && find . -type f ! -name SHA256SUMS -print0 \
        | sort -z | xargs -0 sha256sum > SHA256SUMS

# Remove all build artifacts: the cargo target directory and the staged
# distribution binaries in dist/.
clean:
    cargo clean
    rm -rf dist
