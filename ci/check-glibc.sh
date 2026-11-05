#!/usr/bin/env bash
# Fail when a Linux binary requires a glibc newer than the supported baseline.
#
# Steam Deck (SteamOS) and older LTS distributions ship an older glibc; a
# binary linked against the CI runner's libc refuses to start there with
# "version `GLIBC_2.44' not found".
set -euo pipefail

binary=${1:?usage: check-glibc.sh <binary> [max-version]}
allowed=${2:-2.35}

versions=$(objdump -T "$binary" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV)
highest=$(printf '%s\n' "$versions" | tail -1)

if [ -z "$highest" ]; then
    echo "error: $binary does not link against glibc?" >&2
    exit 1
fi

# The lexically greatest version after sort -V must not exceed the baseline.
if [ "$(printf '%s\n%s\n' "$allowed" "$highest" | sort -V | tail -1)" != "$allowed" ]; then
    echo "error: $binary requires glibc $highest, baseline is $allowed" >&2
    exit 1
fi
echo "ok: $binary requires at most glibc $highest (baseline $allowed)"
