---
name: git-workflow
description: Use when starting, committing or finishing any change in JadeForge, or when the worktree is dirty; every task runs on its own branch and ends with a clean worktree.
---

# Task branch workflow

`main` is integration only. Never edit files, stage or commit on it. A task of
any size gets its own branch from the latest `origin/main`, and it is finished
only when everything is committed and `just finish` passes.

## Start

1. Look before touching anything:

   ```sh
   git status --short --branch
   git branch --show-current
   ```

   A dirty worktree at the start is not yours to throw away. Do not use
   `git checkout -- .`, `git reset --hard`, `git clean` or `git stash` to make
   it clean. Commit the work on its proper branch if the user says so, or ask.

2. Install the hooks once per clone (this sets `core.hooksPath=.githooks`):

   ```sh
   just setup
   git config --local core.hooksPath   # must print .githooks
   ```

3. Create the branch. `just start` fetches `origin/main`, refuses a dirty
   worktree and validates the name:

   ```sh
   just start <type>/<slug>
   ```

   `type` is `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`,
   `chore` or `release`; use a lower-case kebab-case slug
   (`fix/zero-length-locstrings`, `feat/style-upgrade-orbs`). Never branch from
   a stale local `main`, and never work directly on `main`: the tracked
   pre-commit hook refuses commits there.

## Work and commit

- Keep one logical change per branch. If a second, unrelated change shows up,
  ask the user whether it belongs on this branch or on a new one.
- Commit as the work reaches coherent states. Stage only the intended files,
  review `git diff --cached`, and use imperative lower-case subjects
  (`fix zero-length locstring parsing`) with a body that explains *why* when
  it is not obvious.
- Do not amend, rebase or force-push a branch that was already pushed unless
  the user asks.
- Run the relevant tests while working. Do not hide failures in the final
  commit.

## Finish

In this order:

1. Update the documentation the change requires (`docs/FORMAT.md`, `README.md`,
   `CHANGELOG.md`, ...), then run the full gate:

   ```sh
   just check
   ```

2. Review what the branch contains:

   ```sh
   git status --short
   git log --oneline origin/main..HEAD
   git diff --stat origin/main...HEAD
   ```

3. Commit anything still outstanding. After the last commit,
   `git status --porcelain` must print nothing. Ignored build output such as
   `target/` or `scratch/` does not count.

4. Run the hand-over guard:

   ```sh
   just finish
   ```

   It fails if the branch is `main`, the worktree is dirty or the branch has no
   commits beyond `origin/main`, and otherwise reports the commit count.

5. Leave the task branch checked out. Only push or open a pull request when the
   user asks:

   ```sh
   git push -u origin <branch>
   ```

   The branch lands on `main` through a pull request (preferred) or a deliberate
   merge by the user; do not merge it yourself. After the merge, delete the
   branch and start the next task from `origin/main`.

## Do not

- Do not commit on `main`, even if the change is tiny; `pre-commit` refuses it.
- Do not use `git commit --no-verify` to bypass the hook. If the hook is in the
  way, the task belongs on a branch.
- Do not leave modified or untracked files behind after a task.
- Do not discard uncommitted work you did not create.
- Do not push, force-push or open a pull request unless the user asked.
