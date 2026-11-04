#!/usr/bin/env bash
# Reports lints that are not gated yet. Always exits 0: this is an audit
# report, not a build gate.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo clippy --workspace --all-targets --message-format=json -- \
    -W clippy::expect_used \
    -W clippy::indexing_slicing \
    -W clippy::arithmetic_side_effects \
    -W clippy::nursery 2>/dev/null \
| python3 -c '
import collections, json, sys

counts = collections.Counter()
for line in sys.stdin:
    try:
        message = json.loads(line)
    except json.JSONDecodeError:
        continue
    if message.get("reason") != "compiler-message":
        continue
    body = message["message"]
    if body.get("level") not in ("warning", "error"):
        continue
    code = (body.get("code") or {}).get("code") or "unknown"
    counts[code] += 1

total = sum(counts.values())
print(f"{total} strict diagnostics")
for code, number in counts.most_common():
    print(f"{number:5}  {code}")
'
