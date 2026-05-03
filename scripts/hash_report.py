#!/usr/bin/env python3
"""Body-only SHA-256 for YAML-front-mattered markdown reports.

Usage: scripts/hash_report.py <file.md> [<file.md>...]

Strips a leading ``---\\n...\\n---\\n`` YAML front-matter block (if any),
then sha256 over the remaining UTF-8 bytes. Prints "<hash>  <path>" per
file (matches sha256sum's output format).

This is the canonical body-hash function for the 9-anchor regression
gate. Backtest reports must keep all run-varying metadata
(generated:, wall_clock_s, host, pid, git_commit, data_source) in the
front-matter so it is excluded from the body hash.
"""
import hashlib
import re
import sys
from pathlib import Path

FRONTMATTER_RE = re.compile(r"^---\n.*?\n---\n", re.DOTALL)


def body_hash(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    match = FRONTMATTER_RE.match(text)
    body = text[match.end():] if match else text
    return hashlib.sha256(body.encode("utf-8")).hexdigest()


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: hash_report.py <path> [<path>...]", file=sys.stderr)
        return 2
    rc = 0
    for arg in argv[1:]:
        path = Path(arg)
        if not path.is_file():
            print(f"missing  {arg}", file=sys.stderr)
            rc = 1
            continue
        print(f"{body_hash(path)}  {arg}")
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv))
