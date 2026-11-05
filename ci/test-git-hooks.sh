#!/usr/bin/env bash
# Smoke-test the tracked git hooks: a direct commit on main must be refused and
# a commit on a task branch must succeed. Runs in a throwaway repository, so it
# never touches the checkout it is called from.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
hook="$root/.githooks/pre-commit"

if [[ ! -x "$hook" ]]; then
    echo "test-git-hooks: $hook is missing or not executable" >&2
    exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

git -C "$tmp" init -q -b main
git -C "$tmp" config user.name "JadeForge test"
git -C "$tmp" config user.email "test@example.invalid"
git -C "$tmp" config core.hooksPath "$root/.githooks"

printf 'one\n' >"$tmp/file"
git -C "$tmp" add file

if git -C "$tmp" commit -qm "direct commit on main" 2>"$tmp/err"; then
    echo "test-git-hooks: the hook allowed a commit on main" >&2
    exit 1
fi

if ! grep -q "direct commits on 'main' are disabled" "$tmp/err"; then
    echo "test-git-hooks: unexpected refusal message:" >&2
    cat "$tmp/err" >&2
    exit 1
fi

git -C "$tmp" switch -q -c feat/test-branch
git -C "$tmp" commit -qm "commit on a task branch"

branch=$(git -C "$tmp" rev-parse --abbrev-ref HEAD)
if [[ "$branch" != "feat/test-branch" ]]; then
    echo "test-git-hooks: expected the task branch, found '$branch'" >&2
    exit 1
fi

echo "test-git-hooks: ok (main refused, task branch accepted)"
