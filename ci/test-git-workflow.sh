#!/usr/bin/env bash
# Smoke-test the task-branch workflow in throwaway repositories: the
# pre-commit hook refuses main, start validates the branch and the worktree,
# and finish rejects a dirty or empty branch and accepts a committed one.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
workflow="$root/ci/git-workflow.sh"
hook="$root/.githooks/pre-commit"

for file in "$workflow" "$hook"; do
    if [[ ! -x "$file" ]]; then
        echo "test-git-workflow: $file is missing or not executable" >&2
        exit 1
    fi
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

new_repo() {
    git init -q -b main "$1"
    git -C "$1" config user.name "JadeForge test"
    git -C "$1" config user.email "test@example.invalid"
}

fail() {
    echo "test-git-workflow: $1" >&2
    exit 1
}

expect_failure() {
    local label=$1 dir=$2
    shift 2
    if (cd "$dir" && "$@") >/dev/null 2>&1; then
        fail "$label was not refused"
    fi
}

expect_success() {
    local label=$1 dir=$2
    shift 2
    if ! (cd "$dir" && "$@") >/dev/null 2>&1; then
        fail "$label failed"
    fi
}

# The hook: main refuses a direct commit, a task branch accepts it.
repo="$tmp/hook"
new_repo "$repo"
git -C "$repo" config core.hooksPath "$root/.githooks"
printf 'one\n' >"$repo/file"
git -C "$repo" add file
if git -C "$repo" commit -qm "direct commit on main" 2>"$tmp/hook-error"; then
    fail "the hook allowed a direct commit on main"
fi
if ! grep -q "direct commits on 'main' are disabled" "$tmp/hook-error"; then
    cat "$tmp/hook-error" >&2
    fail "the hook refused main with an unexpected message"
fi
git -C "$repo" switch -q -c feat/test-branch
git -C "$repo" commit -qm "commit on a task branch"

# start/finish, against a real (bare) origin.
repo="$tmp/workflow"
new_repo "$repo"
printf 'base\n' >"$repo/file"
git -C "$repo" add file
git -C "$repo" commit -qm "base"
git init -q --bare "$tmp/origin.git"
git -C "$repo" remote add origin "$tmp/origin.git"
git -C "$repo" push -q -u origin main

expect_failure "start with an invalid name" "$repo" "$workflow" start no-slash

printf 'dirty\n' >>"$repo/file"
expect_failure "start with a dirty worktree" "$repo" "$workflow" start feat/next
git -C "$repo" restore file

expect_success "start from a clean worktree" "$repo" "$workflow" start feat/next
expect_failure "finish on a branch with no commits" "$repo" "$workflow" finish

printf 'work\n' >>"$repo/file"
git -C "$repo" add file
git -C "$repo" commit -qm "work"

expect_success "finish on a committed branch" "$repo" "$workflow" finish

printf 'dirty\n' >>"$repo/file"
expect_failure "finish with a dirty worktree" "$repo" "$workflow" finish
git -C "$repo" restore file

git -C "$repo" switch -q main
expect_failure "finish on main" "$repo" "$workflow" finish

echo "test-git-workflow: ok (hook, start and finish guards)"
