#!/usr/bin/env bash
# Start and finish task branches. Called by `just start` and `just finish`; the
# policy lives in AGENTS.md ("Branch workflow").
set -euo pipefail

usage() {
    echo "usage: $0 start <type>/<slug>" >&2
    echo "       $0 finish" >&2
    exit 2
}

start() {
    local branch=${1:-}
    if [[ -z "$branch" ]]; then
        usage
    fi
    case "$branch" in
    feat/* | fix/* | docs/* | test/* | refactor/* | perf/* | build/* | ci/* | chore/* | release/*) ;;
    *)
        echo "start: branch must look like <type>/<slug>, e.g. fix/save-load" >&2
        echo "start: allowed types: feat fix docs test refactor perf build ci chore release" >&2
        exit 1
        ;;
    esac
    if [[ ! "$branch" =~ ^[a-z]+/[a-z0-9][a-z0-9._-]*$ ]]; then
        echo "start: '$branch' is not lower-case kebab-case after the type prefix" >&2
        exit 1
    fi
    if [[ -n "$(git status --porcelain)" ]]; then
        echo "start: the worktree has uncommitted changes; commit them before switching" >&2
        git status --short >&2
        exit 1
    fi
    if ! git fetch --quiet origin main; then
        echo "start: warning: could not fetch origin/main; using the cached ref" >&2
    fi
    local base=origin/main
    if ! git rev-parse --verify --quiet "$base" >/dev/null; then
        base=main
    fi
    if ! git rev-parse --verify --quiet "$base" >/dev/null; then
        echo "start: neither origin/main nor main exists; fetch the repository first" >&2
        exit 1
    fi
    git switch --create "$branch" "$base"
    printf 'start: on %s (base %s %s)\n' "$branch" "$base" "$(git rev-parse --short "$base")"
}

finish() {
    local branch
    branch=$(git symbolic-ref --quiet --short HEAD || true)
    if [[ -z "$branch" ]]; then
        echo "finish: HEAD is detached; check out the task branch first" >&2
        exit 1
    fi
    case "$branch" in
    main | master)
        echo "finish: '$branch' is the integration branch, not a task branch" >&2
        exit 1
        ;;
    esac
    local dirty
    dirty=$(git status --porcelain)
    if [[ -n "$dirty" ]]; then
        echo "finish: the worktree is not clean; commit the task's work on $branch" >&2
        printf '%s\n' "$dirty" >&2
        exit 1
    fi
    local base=origin/main
    if ! git rev-parse --verify --quiet "$base" >/dev/null; then
        base=main
    fi
    if ! git rev-parse --verify --quiet "$base" >/dev/null; then
        echo "finish: neither origin/main nor main exists; fetch the repository first" >&2
        exit 1
    fi
    local ahead
    ahead=$(git rev-list --count "$base..HEAD")
    if [[ "$ahead" -eq 0 ]]; then
        echo "finish: $branch has no commits beyond $base" >&2
        exit 1
    fi
    if [[ "$(git config --local core.hooksPath || true)" != ".githooks" ]]; then
        echo "finish: warning: git hooks are not installed; run: just setup" >&2
    fi
    printf 'finish: %s is clean and %s commit(s) ahead of %s\n' "$branch" "$ahead" "$base"
    printf 'finish: next: push the branch and open a pull request, or merge it\n'
}

case "${1:-}" in
start)
    shift
    start "$@"
    ;;
finish)
    shift
    finish
    ;;
*) usage ;;
esac
