# Security policy

## Supported versions

JadeForge is pre-1.0. Security fixes are made on the latest tagged release and
the `main` branch; older releases are not maintained.

## Reporting a vulnerability

Please do not open a public issue for a vulnerability. Report it privately to
the maintainer listed in `Cargo.toml` (or through GitHub's private vulnerability
reporting for the repository), including:

- the affected version,
- a minimal savegame or input that reproduces the problem (do not send a retail
  savegame unless it is necessary; a synthetic mutation is preferred),
- the expected and actual behaviour,
- whether a malicious savegame, game installation or local attacker is needed.

You should receive an acknowledgement within a few working days. Please allow
time for a fix and a coordinated disclosure before publishing details.

## Scope

The editor parses untrusted savegames and game-data files. Crashes, hangs,
unbounded allocations, path traversal, symlink following, writing outside the
selected savegame and any way to make the editor write an unverified savegame
are security issues. The game installation is read-only by design; a bug that
opens it for writing is in scope.

The project does not implement its own cryptography. Release signing and
notarisation depend on platform credentials and are documented in
`ci/README.md`.

## Secrets

Never hardcode secrets or other sensitive data in the repository. API keys,
tokens, passwords, private keys, certificates and signing credentials belong in
the environment or in untracked local files. CI reads them from repository
secrets. Tests, fixtures, examples and documentation use obvious placeholders,
never real-looking credentials. A committed secret must be rotated and removed
from history, not merely deleted in a later commit.
