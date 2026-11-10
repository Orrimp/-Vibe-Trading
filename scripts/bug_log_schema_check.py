#!/usr/bin/env python3
# /// script
# requires-python = ">=3.9"
# ///
"""bug_log_schema_check.py — the bug-log's own declared Status schema, enforced from a mark.

bug-log `#195` (2026-11-10): `docs/dev-notes/bug-log.md` declares a Status schema in its
own header — four values, and a landing commit for every shipped row — and nothing read
it. Measured over all 139 entries the day it was found: 50 had no `**Status**:` line at
all. This gate enforces that schema from `HIGH_WATER_MARK` on (`#195` option 3). The mark
is ONE declared number, not a list of exemptions; the header must declare the same number
("Enforced from `#NN`"), and the PASS line counts the entries it leaves out.

THE GATED REGION starts at the first `###` heading whose id is >= the mark and runs to the
end of the file. The exemption is by POSITION as well as by id: inside the region every
`###` heading is an entry, and ids strictly increase, so an old id cannot be re-opened at
the bottom of the file and skip every check (review finding BYP-1).

Per entry in the region — the form  ### `#NN` — title
  E1 no-status          the first non-blank line after the heading is not `**Status**: ...`
  E2 status-value       the Status value — the FIRST word after `**Status**:` — is not one the
                        header's `Status values` list declares (case-sensitive), or is
                        written as a transition (`open` → `fixed`)
  E3 no-landing-commit  a `fixed` / `partial-fix` row cites no commit in its Status paragraph
  E4 commit-not-landed  a commit a `fixed` / `partial-fix` row cites is not an ancestor of HEAD
                        (or of MERGE_HEAD while a merge is being concluded)
  E5 order              ids do not strictly increase down the region
  E6 heading-format     a `###` heading in the region is not in the entry form, or a heading of
                        another level STARTS with an id newer than every entry so far (a new
                        entry at the wrong level, which E1-E5 would never see)
  E7 placeholder        a `fixed` / `partial-fix` Status paragraph says "this commit" (it rots:
                        `#192`, `#194` — a commit cannot cite itself; name the hash in a follow-up)

A COMMIT CITATION is a backtick span that STARTS with 7-40 lowercase hex characters and
continues with the closing backtick or whitespace (`1a2b3c4d`, `1a2b3c4d fix(x): y`).
All-digit spans (`42762269706`, `20261110`) are run ids and dates, never citations. To
mention other hex in a Status paragraph, label it inside the span (`body 9f8e7d6c`).
The Status paragraph is the Status line plus every following line up to the first blank
line or heading, so a citation that wraps onto the next line still counts.

The vocabulary and the mark are PARSED from the file's header, not copied here, so the file
and the gate cannot disagree about either without the gate saying so.

Exit codes:
  0  every entry in the region conforms; the PASS line on stdout says what was read
  1  violations, one line each on stderr
  2  the gate cannot measure: file missing, not UTF-8, not in the index (--staged), vocabulary
     unparseable, a terminal value or the mark undeclared by the header, no entry at or above
     the mark, an unclosed code fence, a shallow clone, or no git

Usage:
    python3 scripts/bug_log_schema_check.py              # the working tree (CI: the checkout)
    python3 scripts/bug_log_schema_check.py --staged     # the index being committed (the hook)
    python3 scripts/bug_log_schema_check.py --bug-log PATH --repo PATH
    python3 scripts/bug_log_schema_check.py --self-test

KNOWN RESIDUALS (measured by the 2026-11-10 review, accepted, each fails LOUDLY later):
  - `git commit --amend` that folds a Status citation of HEAD into HEAD itself passes here (at
    pre-commit time it is indistinguishable from citing the previous commit); the amended-away
    hash then fails E4 on the next commit and in CI.
  - raw `<h3>` HTML, setext headings (which cannot be level 3) and HTML comments are not parsed.
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

REPO_ROOT = Path(__file__).resolve().parent.parent
BUG_LOG = REPO_ROOT / "docs" / "dev-notes" / "bug-log.md"

# `#189` is where the convention has been followed deliberately: every entry from it on
# carries a Status line (`#189`/`#190` only after `cd0f58e9` repaired them), and `#180`-`#188`
# carry none at all. Raising this number exempts entries; it is a reviewed diff, not a list,
# and the header's "Enforced from `#NN`" must move with it or the gate exits 2.
HIGH_WATER_MARK = 189

# The values that assert something LANDED, so must name what landed. Checked against the
# header's declared vocabulary on every run: if the header stops declaring one, exit 2.
TERMINAL = ("fixed", "partial-fix")

ATX_RE = re.compile(r"^ {0,3}(#{1,6})(?=[ \t]|$)(.*)$")
ENTRY_RE = re.compile(r"^### `#(\d+)` — \S")
# Heading TEXT that starts with an id (optionally `Bug `, optionally backticked). Prose
# sub-headings that merely mention one ("#### What `#188` fixed") do not match.
ID_FIRST_RE = re.compile(r"^(?:[Bb]ug\s+)?`?#(\d+)")
FENCE_RE = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")
STATUS_RE = re.compile(r"^\*\*Status\*\*:")
VALUE_RE = re.compile(r"[A-Za-z][A-Za-z-]*")
CITE_RE = re.compile(r"`([0-9a-f]{7,40})(?=[`\s])")
PLACEHOLDER_RE = re.compile(r"\bthis commit\b", re.IGNORECASE)
TRANSITION_MARKS = ("→", "->", "=>")
VOCAB_LINE_RE = re.compile(r"\*\*Status values\*\*:")
VOCAB_BULLET_RE = re.compile(r"^\s+[-*+]\s")
VOCAB_ITEM_RE = re.compile(r"^\s+[-*+]\s+`([A-Za-z][A-Za-z-]*)`(?=\s|$)")
MARK_DECL_RE = re.compile(r"Enforced from `#(\d+)`")


class CannotMeasure(Exception):
    """Exit 2: the gate could not read what it claims to check."""


@dataclass
class Entry:
    id: int
    line: int  # 1-based heading line
    first_line: int  # 1-based line of the first non-blank line after the heading (0 = none)
    first_text: str
    status_text: Optional[str]  # the Status paragraph, joined; None when E1


@dataclass
class Report:
    gated: List[Entry] = field(default_factory=list)
    below: int = 0
    violations: List[str] = field(default_factory=list)
    values: Dict[str, int] = field(default_factory=dict)
    citations: int = 0
    distinct: int = 0


def _clean_env() -> Dict[str, str]:
    """`os.environ` minus every `GIT_*` variable except GIT_CEILING_DIRECTORIES.

    git exports GIT_INDEX_FILE, GIT_CONFIG_PARAMETERS and others to hooks (and GIT_DIR in a
    linked worktree), and under `git commit -a` (or `commit <paths>`) GIT_INDEX_FILE is an
    ABSOLUTE path to the real repository's `index.lock` — measured 2026-11-10, git 2.50.1.
    Inherited, it aims this gate's git calls, and the self-test's `git add` in a throwaway
    repo, at the commit being made. The repository is the one `--repo` names, never the
    ambient environment. GIT_CEILING_DIRECTORIES is kept: it can only STOP repository
    discovery, never redirect it, and the self-test uses it to make "not a repository" true.
    The ONE call that deliberately keeps the ambient environment is the --staged read.
    """
    return {
        k: v
        for k, v in os.environ.items()
        if not k.startswith("GIT_") or k == "GIT_CEILING_DIRECTORIES"
    }


def _git(repo: Path, *args: str, ambient: bool = False) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(
            ["git", "-C", str(repo), *args],
            capture_output=True,
            env=None if ambient else _clean_env(),
        )
    except FileNotFoundError as exc:
        raise CannotMeasure(f"git is not installed ({exc})") from exc


def _out(proc: subprocess.CompletedProcess) -> str:
    return proc.stdout.decode("utf-8", "replace").strip()


def _err(proc: subprocess.CompletedProcess) -> str:
    return (proc.stderr or b"").decode("utf-8", "replace").strip()


def _fence_mask(lines: List[str]) -> List[bool]:
    """True for every line inside a fenced code block (fence lines included).

    CommonMark's rule, not a toggle: a fence opens and closes only at 0-3 spaces of
    indentation (four or more is an indented code block, not a fence — review finding
    BYP-3), and closes only on a run of the SAME character at least as long as the opener
    with nothing after it. A fence that never closes hides the rest of the file from every
    check, so it is exit 2, not a quiet pass.
    """
    mask: List[bool] = []
    opener: Optional[str] = None
    opened_at = 0
    for i, s in enumerate(lines):
        m = FENCE_RE.match(s)
        if opener is None:
            if m and not (m.group(1)[0] == "`" and "`" in m.group(2)):  # ```inline``` is not a fence
                opener, opened_at = m.group(1), i + 1
                mask.append(True)
            else:
                mask.append(False)
            continue
        mask.append(True)
        if m and m.group(1)[0] == opener[0] and len(m.group(1)) >= len(opener) and not m.group(2).strip():
            opener = None
    if opener is not None:
        raise CannotMeasure(
            f"the code fence opened at L{opened_at} is never closed, so every line after it "
            "would be invisible to this gate"
        )
    return mask


def _heading(s: str) -> Optional[Tuple[int, str, Optional[int]]]:
    """(level, text, id-if-the-text-starts-with-one) for an ATX heading, else None."""
    h = ATX_RE.match(s)
    if not h:
        return None
    text = h.group(2).strip()
    idm = ID_FIRST_RE.match(text)
    return len(h.group(1)), text, (int(idm.group(1)) if idm else None)


def _header_end(lines: List[str], mask: List[bool]) -> int:
    for i, s in enumerate(lines):
        if mask[i]:
            continue
        h = _heading(s)
        if h and h[0] == 3 and h[2] is not None:
            return i
    return len(lines)


def parse_vocabulary(lines: List[str], mask: List[bool]) -> Tuple[str, ...]:
    """The `Status values` list from the header (the region before the first id heading).

    The list ends at the first non-blank line that starts at column 0; inside it, every
    bullet must be ONE backticked word, and every other indented line is a wrapped
    description (review finding FP2/BYP-7: a wrapped description used to end the list and
    silently drop every value after it). A bullet the gate cannot read is exit 2.
    """
    end = _header_end(lines, mask)
    for i in range(end):
        if mask[i] or not VOCAB_LINE_RE.search(lines[i]):
            continue
        vocab: List[str] = []
        for s in lines[i + 1 : end]:
            if not s.strip():
                continue
            if not s[0].isspace():
                break
            if VOCAB_BULLET_RE.match(s):
                m = VOCAB_ITEM_RE.match(s)
                if not m:
                    raise CannotMeasure(
                        f"a `Status values` item the gate cannot read: {s.strip()[:60]!r} "
                        "(each item is ONE backticked word, then its description)"
                    )
                vocab.append(m.group(1))
        if vocab:
            return tuple(vocab)
    raise CannotMeasure(
        "no `**Status values**:` list (`  - `value` — ...` items) in the header "
        "before the first entry; nothing declares the vocabulary"
    )


def _declared_mark(lines: List[str], mask: List[bool], mark: int) -> None:
    end = _header_end(lines, mask)
    found = [int(m.group(1)) for i in range(end) if not mask[i] for m in MARK_DECL_RE.finditer(lines[i])]
    if not found:
        raise CannotMeasure(
            "the header does not say where enforcement starts (\"Enforced from `#NN`\"), so a "
            f"reader cannot know that entries below #{mark} are exempt"
        )
    if any(n != mark for n in found):
        raise CannotMeasure(
            f"the header says \"Enforced from `#{found[0]}`\" but HIGH_WATER_MARK is #{mark}; "
            "move them together (bug-log #191: a citation must name what actually runs)"
        )


def _status_value(status_line: str) -> Tuple[str, str]:
    """(value, what follows it) — the value is the FIRST word after `**Status**:`."""
    rest = STATUS_RE.sub("", status_line, count=1).lstrip(" *`")
    m = VALUE_RE.match(rest)
    if not m:
        return "", rest
    return m.group(0), rest[m.end():]


def _require_full_history(repo: Path) -> None:
    probe = _git(repo, "rev-parse", "--is-shallow-repository")
    if probe.returncode != 0:
        raise CannotMeasure(f"{repo} is not a git repository: {_err(probe) or _out(probe)}")
    if _out(probe) == "true":
        raise CannotMeasure(
            f"{repo} is a SHALLOW clone, so a cited commit may be unresolvable for reasons "
            "outside the bug-log. CI: check out with `fetch-depth: 0`; locally: "
            "`git fetch --unshallow`. Refusing rather than reporting a verdict that depends "
            "on the clone (bug-log #192)."
        )


def _landing_parents(repo: Path) -> Tuple[List[str], bool]:
    """HEAD, plus every MERGE_HEAD while a merge is being concluded (review finding FP6/F2)."""
    parents: List[str] = []
    head = _git(repo, "rev-parse", "--verify", "--quiet", "HEAD^{commit}")
    if head.returncode == 0:
        parents.append(_out(head))
    merging = False
    gp = _git(repo, "rev-parse", "--git-path", "MERGE_HEAD")
    if gp.returncode == 0:
        p = Path(_out(gp))
        p = p if p.is_absolute() else repo / p
        if p.is_file():
            for line in p.read_text(encoding="utf-8", errors="replace").split():
                if re.fullmatch(r"[0-9a-f]{40,64}", line):
                    parents.append(line)
                    merging = True
    return parents, merging


def _resolve(repo: Path, token: str, parents: Sequence[str], merging: bool) -> Optional[str]:
    """None when `token` names a commit that is an ancestor of a landing parent; else why not."""
    full = _git(repo, "rev-parse", "--verify", "--quiet", f"{token}^{{commit}}")
    if full.returncode != 0:
        return "is not a commit in this repository (unknown, ambiguous, or not a commit)"
    sha = _out(full)
    for p in parents:
        anc = _git(repo, "merge-base", "--is-ancestor", sha, p)
        if anc.returncode == 0:
            return None
        if anc.returncode != 1:
            return f"could not be checked: {_err(anc)}"
    where = "HEAD or MERGE_HEAD" if merging else "HEAD"
    return f"is commit {sha[:12]}, which is NOT an ancestor of {where} (has not landed here)"


def check(text: str, repo: Path, mark: int) -> Report:
    lines = text.split("\n")
    mask = _fence_mask(lines)
    vocab = parse_vocabulary(lines, mask)
    missing = [t for t in TERMINAL if t not in vocab]
    if missing:
        raise CannotMeasure(
            f"the header's Status values {list(vocab)} do not declare {missing}, which this "
            "gate treats as terminal (must cite a landing commit); update one or the other"
        )
    _declared_mark(lines, mask, mark)
    _require_full_history(repo)

    rep = Report()
    region = False
    newest: Optional[int] = None
    for i, s in enumerate(lines):
        if mask[i]:
            continue
        h = _heading(s)
        if not h:
            continue
        level, text_, n = h
        if not region:
            if level == 3 and n is not None and n >= mark:
                region = True  # this heading is the first one in the gated region
            elif n is not None and n >= mark:
                rep.violations.append(
                    f"E6 heading-format     #{n} (L{i + 1}): a level-{level} heading starts with an "
                    f"id at or above the mark; an entry is  ### `#{n}` — title"
                )
                continue
            else:
                if level == 3 and n is not None:
                    rep.below += 1
                continue
        if level != 3:
            if n is not None and (newest is None or n > newest):
                rep.violations.append(
                    f"E6 heading-format     #{n} (L{i + 1}): a level-{level} heading starts with "
                    f"#{n}, newer than every entry so far — a new entry is  ### `#{n}` — title"
                )
            continue
        m = ENTRY_RE.match(s)
        if not m:
            rep.violations.append(
                f"E6 heading-format     (L{i + 1}): {s[:70]!r} — in the gated region every ### "
                "heading is an entry, of the form  ### `#NN` — title  (sub-sections are ####)"
            )
            continue
        n = int(m.group(1))
        newest = n if newest is None else max(newest, n)
        j = i + 1
        while j < len(lines) and not lines[j].strip():
            j += 1
        first = lines[j] if j < len(lines) else ""
        entry = Entry(n, i + 1, j + 1 if j < len(lines) else 0, first, None)
        if j < len(lines) and STATUS_RE.match(first):
            para, k = [], j
            while k < len(lines) and lines[k].strip() and not (k > j and ATX_RE.match(lines[k])):
                para.append(lines[k])
                k += 1
            entry.status_text = "\n".join(para)
        rep.gated.append(entry)

    if not region:
        raise CannotMeasure(
            f"no entry numbered #{mark} or above: the mark points at nothing, so this gate "
            "would pass having read nothing"
        )

    parents, merging = _landing_parents(repo)
    seen: Dict[str, Optional[str]] = {}
    prev: Optional[Entry] = None
    for e in rep.gated:
        if prev is not None and e.id <= prev.id:
            if e.id < mark:
                why = (f"#{e.id} is below the mark (#{mark}): an old id cannot be re-opened after "
                       "the gated entries — open a new #NN, or append a dated blockquote inside "
                       "the original entry")
            else:
                why = "ids must strictly increase down the file (a duplicate or a reordering)"
            rep.violations.append(
                f"E5 order              #{e.id} (L{e.line}) follows #{prev.id} (L{prev.line}): {why}"
            )
        prev = e
        if e.status_text is None:
            shown = e.first_text.strip()[:60] or "<end of file>"
            rep.violations.append(
                f"E1 no-status          #{e.id} (L{e.line}): first non-blank line after the "
                f"heading is {shown!r}, not `**Status**: ...`"
            )
            continue
        value, after = _status_value(e.status_text.split("\n", 1)[0])
        rep.values[value] = rep.values.get(value, 0) + 1
        if value not in vocab:
            hint = ""
            if value.lower() in vocab:
                hint = f" — values are case-sensitive; the header declares `{value.lower()}`"
            rep.violations.append(
                f"E2 status-value       #{e.id} (L{e.first_line}): {value or '<none>'!r} is not "
                f"a declared Status value {list(vocab)}{hint}"
            )
        elif after.lstrip(" *`").startswith(TRANSITION_MARKS):
            rep.violations.append(
                f"E2 status-value       #{e.id} (L{e.first_line}): `{value}` is written as a "
                "transition; the value is the FIRST word, so write the CURRENT one first and the "
                "history in prose"
            )
        if value not in TERMINAL:
            continue
        cites = [t for t in CITE_RE.findall(e.status_text) if not t.isdigit()]
        if not cites:
            notes = []
            if any(t.isdigit() for t in CITE_RE.findall(e.status_text)):
                notes.append("all-digit spans are run ids/dates, not hashes")
            if re.search(r"`[0-9A-F]{7,40}(?=[`\s])", e.status_text) and re.search(r"[A-F]", e.status_text):
                notes.append("hashes are lowercase")
            rep.violations.append(
                f"E3 no-landing-commit  #{e.id} (L{e.first_line}): `{value}` names no landing "
                "commit in its Status paragraph (cite it in backticks, e.g. `1a2b3c4d`)"
                + (f" — {'; '.join(notes)}" if notes else "")
            )
        if PLACEHOLDER_RE.search(e.status_text):
            rep.violations.append(
                f"E7 placeholder        #{e.id} (L{e.first_line}): the Status paragraph says "
                "\"this commit\"; a commit cannot cite itself, and the phrase rots (#192, #194) — "
                "name the hash in a follow-up commit"
            )
        for h_ in cites:
            rep.citations += 1
            if h_ not in seen:
                seen[h_] = _resolve(repo, h_, parents, merging)
            why = seen[h_]
            if why:
                rep.violations.append(
                    f"E4 commit-not-landed  #{e.id} (L{e.first_line}): `{h_}` {why}"
                )
    rep.distinct = len(seen)
    return rep


def _read(bug_log: Path, repo: Path, staged: bool) -> str:
    if staged:
        try:
            rel = bug_log.resolve().relative_to(repo.resolve()).as_posix()
        except ValueError as exc:
            raise CannotMeasure(f"{bug_log} is not inside {repo}") from exc
        # The ONE git call that keeps the ambient environment: inside a hook GIT_INDEX_FILE
        # names the index being committed, and that is exactly what --staged must read.
        blob = _git(repo, "cat-file", "blob", f":{rel}", ambient=True)
        if blob.returncode != 0:
            raise CannotMeasure(f"{rel} is not in the index: {_err(blob)}")
        data = blob.stdout
    else:
        if not bug_log.is_file():
            raise CannotMeasure(f"{bug_log} does not exist")
        data = bug_log.read_bytes()
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise CannotMeasure(f"{bug_log} is not valid UTF-8 (byte {exc.start}: {exc.reason})") from exc


def run(bug_log: Path, repo: Path, mark: int, staged: bool = False, out=None, err=None) -> int:
    out = out if out is not None else sys.stdout
    err = err if err is not None else sys.stderr
    source = "staged index" if staged else "working tree"
    try:
        rep = check(_read(bug_log, repo, staged), repo, mark)
    except CannotMeasure as exc:
        print(f"bug-log schema ({source}): CANNOT MEASURE — {exc}", file=err)
        return 2
    if rep.violations:
        print(f"bug-log schema ({source}): FAIL — {len(rep.violations)} violation(s) in {bug_log}", file=err)
        for v in rep.violations:
            print(f"  {v}", file=err)
        print(
            "  Schema: the file's own `## Conventions` header, enforced from the mark "
            "(HIGH_WATER_MARK in scripts/bug_log_schema_check.py); bug-log #195.",
            file=err,
        )
        return 1
    ids = [e.id for e in rep.gated]
    values = " · ".join(f"{k} {v}" for k, v in sorted(rep.values.items()))
    print(
        f"bug-log schema ({source}): PASS — {len(ids)} entr{'y' if len(ids) == 1 else 'ies'} "
        f"gated (#{ids[0]}–#{ids[-1]}, mark #{mark}) · {rep.below} id heading(s) below the mark, "
        f"not gated · {values} · {rep.citations} commit citation(s), {rep.distinct} distinct, "
        "all landed",
        file=out,
    )
    return 0


# ─────────────────────────────── self-test ───────────────────────────────

_DEFAULT_VOCAB = ("fixed", "partial-fix", "open", "wontfix")
_PIN = ["-c", "user.email=t@t", "-c", "user.name=t", "-c", "commit.gpgsign=false",
        "-c", "core.hooksPath=/dev/null", "-c", "init.defaultObjectFormat=sha1"]
# Fixture commits are DETERMINISTIC: fixed dates + fixed content + SHA-1 give the same hashes
# on every machine. They used to be fresh per run, and an 8-character prefix is all digits
# with probability (10/16)^8 = 2.3% — which this gate rightly refuses to read as a citation —
# so with ~5 cited prefixes a run the self-test failed 3 times in 30 (measured 2026-11-10).
_FIXED_DATES = {"GIT_AUTHOR_DATE": "2026-01-01T00:00:00+0000",
                "GIT_COMMITTER_DATE": "2026-01-01T00:00:00+0000"}


def _doc(*entries: str, vocab=_DEFAULT_VOCAB, mark: Optional[int] = 10, items: Optional[str] = None) -> str:
    body = items if items is not None else "\n".join(f"  - `{v}` — meaning." for v in vocab)
    decl = f"- **Enforced from `#{mark}`** by the gate.\n" if mark is not None else ""
    return (
        "# Bug log\n\n## Conventions\n\n- **Status values**:\n" + body + "\n"
        "- **Status anchor**: every `fixed`/`partial-fix` row links its landing commit.\n"
        + decl + "\n## Bugs\n\n" + "\n".join(entries)
    )


def _entry(n: int, status: Optional[str], body: str = "Body text.", heading: Optional[str] = None) -> str:
    head = heading if heading is not None else f"### `#{n}` — title {n}"
    parts = [head, ""]
    if status is not None:
        parts += [status, ""]
    parts += [body, ""]
    return "\n".join(parts)


def _g(root: Path, *a: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *_PIN, *a], capture_output=True, text=True, check=True,
        env={**_clean_env(), **_FIXED_DATES},
    ).stdout.strip()


def _fixture_repo(root: Path) -> Tuple[str, str]:
    """Two commits on the default branch, one on a side branch: (landed, not landed)."""
    _g(root, "init", "-q")
    for name in ("a", "b"):
        (root / name).write_text(name, encoding="utf-8")
        _g(root, "add", name)
        _g(root, "commit", "-qm", name)
    landed = _g(root, "rev-parse", "HEAD")
    _g(root, "checkout", "-q", "-b", "side")
    (root / "c").write_text("c", encoding="utf-8")
    _g(root, "add", "c")
    _g(root, "commit", "-qm", "c")
    unlanded = _g(root, "rev-parse", "HEAD")
    _g(root, "checkout", "-q", "-")
    return landed, unlanded


def _tree_snapshot(root: Path) -> Dict[str, bytes]:
    return {p.relative_to(root).as_posix(): p.read_bytes() for p in sorted(root.rglob("*")) if p.is_file()}


def self_test() -> int:
    import io

    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td).resolve()
        # HERMETIC: the hook hands this process ITS GIT_INDEX_FILE (absolute under `commit -a`),
        # and the --staged read honours that variable on purpose — so without this the
        # self-test's own staged probes read the commit being made instead of their fixture.
        # Found by driving the real hook through `git commit -a` (2026-11-10): P12a/P12c went
        # exit 2 and blocked every `commit -a`. P11/P12g set their GIT_* explicitly; P14 pins this.
        saved = dict(os.environ)
        try:
            for key in [k for k in os.environ if k.startswith("GIT_")]:
                del os.environ[key]
            # Make "not a repository" TRUE even when TMPDIR sits inside a work tree (review
            # FP7), and for a directory NESTED in the fixture repo: that is what pins the
            # ceiling surviving `_clean_env()` (probe P10f).
            os.environ["GIT_CEILING_DIRECTORIES"] = os.pathsep.join([str(tmp), str(tmp / "repo")])
            return _self_test_in(tmp, io)
        finally:
            os.environ.clear()
            os.environ.update(saved)


def _self_test_in(tmp: Path, io) -> int:
    repo = tmp / "repo"
    repo.mkdir()
    landed, unlanded = _fixture_repo(repo)
    L, U = landed[:8], unlanded[:8]
    shallow = tmp / "shallow"
    # `-c core.hooksPath` both ways: the user's global hooks must not run (review F6), and the
    # file URL is built by as_uri() so a `%` in TMPDIR is encoded, not decoded.
    subprocess.run(
        ["git", "-c", "core.hooksPath=/dev/null", "clone", "-q", "-c", "core.hooksPath=/dev/null",
         "--depth", "1", repo.as_uri(), str(shallow)],
        capture_output=True, text=True, check=True, env=_clean_env(),
    )
    merging = tmp / "merging"
    merging.mkdir()
    _, side3 = _fixture_repo(merging)
    _g(merging, "merge", "--no-ff", "--no-commit", "side")
    notgit = repo / "nested-not-a-repo"  # inside a work tree: only the ceiling makes it "not a repo"
    notgit.mkdir()

    D, E = _doc, _entry
    ok_open = "**Status**: `open` — found."
    fixed_ok = f"**Status**: **`fixed` — `{L}`.**"

    def at(n: int) -> str:  # an E-code naming entry n, NOT the "#10" in the FAIL trailer
        return f"#{n} (L"

    # P0 the fixture is well-formed: every prefix a probe cites must contain a hex LETTER,
    # or the probe would be testing the all-digit rule instead of what its name says.
    cited = {"L": L, "U": U, "landed[:7]": landed[:7], "landed[:10]": landed[:10], "side3": side3[:8]}
    bad_prefixes = [f"{k}={v}" for k, v in cited.items() if v.isdigit()]
    if bad_prefixes:
        print(f"  FAIL P0 fixture well-formed: all-digit cited prefixes {bad_prefixes}")
        print("bug-log schema self-test: FAIL — the fixture cannot exercise its probes")
        return 1
    print(f"  ok   P0 fixture well-formed (deterministic: landed={landed[:12]})")

    # (name, text, mark, repo, expected rc, substrings that MUST appear, that must NOT)
    probes: List[tuple] = [
        # ── P1 the clean case, and the success path says what it read ──
        ("P1a clean", D(E(9, None), E(10, fixed_ok), E(11, ok_open)), 10, repo, 0,
         ["PASS", "(working tree)", "2 entries gated", "#10–#11", "1 id heading(s) below the mark",
          "fixed 1", "open 1", "1 commit citation(s), 1 distinct"], []),
        ("P1b counters", D(E(9, None), E(9, None, heading="#### `#9` a sub-section"),
                           E(10, f"**Status**: `fixed` — `{L}` + `{L}`."),
                           E(11, f"**Status**: `fixed` — `{L}` and `{landed[:10]}`.")), 10, repo, 0,
         ["4 commit citation(s), 2 distinct", "1 id heading(s) below the mark", "fixed 2"], []),
        # ── P2 the mark is PINNED: P1a's ungated #9 fails the moment the mark covers it ──
        ("P2 mark pinned", D(E(9, None), E(10, fixed_ok), E(11, ok_open), mark=9), 9, repo, 1,
         ["E1 no-status", at(9)], [at(10), at(11)]),
        # ── P3 E1 ──
        ("P3a status not first", D(E(10, None, body="**Found** today.\n\n**Status**: `open`.")), 10, repo, 1,
         ["E1 no-status", at(10), "Found"], []),
        ("P3b colon inside the bold", D(E(10, "**Status:** `open`.")), 10, repo, 1, ["E1 no-status", at(10)], []),
        # ── P4 E2: derived, case-sensitive, first word, no transitions ──
        ("P4a undeclared value", D(E(10, "**Status**: **RESOLVED** — done.")), 10, repo, 1,
         ["E2 status-value", at(10), "'RESOLVED'"], []),
        ("P4b value derived from header", D(E(10, "**Status**: **RESOLVED** — done."),
                                            vocab=_DEFAULT_VOCAB + ("RESOLVED",)), 10, repo, 0,
         ["PASS", "RESOLVED 1"], []),
        ("P4c case-sensitive", D(E(10, f"**Status**: **FIXED** — `{L}`.")), 10, repo, 1,
         ["E2 status-value", "'FIXED'", "case-sensitive", "declares `fixed`"], []),
        ("P4d a transition is not a value", D(E(10, f"**Status**: `open` → `fixed` — `{L}`.")), 10, repo, 1,
         ["E2 status-value", "transition"], []),
        ("P4e a historical mention is fine", D(E(10, f"**Status**: **`fixed` — `{L}`.** *(It said `open` for an hour.)*")),
         10, repo, 0, ["PASS", "fixed 1"], []),
        # ── P5 E3 / E7: terminal rows must cite, and must not cite themselves ──
        ("P5a fixed, no hash", D(E(10, "**Status**: `fixed` 2026-11-10.")), 10, repo, 1,
         ["E3 no-landing-commit", at(10), "`fixed`"], []),
        ("P5b partial-fix, no hash", D(E(10, "**Status**: `partial-fix` — half.")), 10, repo, 1,
         ["E3 no-landing-commit", "`partial-fix`"], []),
        ("P5c wontfix, no hash", D(E(10, "**Status**: `wontfix` — by design.")), 10, repo, 0, ["PASS", "wontfix 1"], []),
        ("P5d placeholder on a terminal row", D(E(10, f"**Status**: `fixed` — `{L}` + this commit (the rest).")),
         10, repo, 1, ["E7 placeholder", at(10), "this commit"], []),
        ("P5e placeholder on an open row is prose", D(E(10, "**Status**: `open` — this commit only records it.")),
         10, repo, 0, ["PASS"], []),
        # ── P6 what a citation IS ──
        ("P6a wrapped citation counts", D(E(10, f"**Status**: **`fixed`** — landed in\n`{L}` today.")), 10, repo, 0, ["PASS"], []),
        ("P6b next-paragraph citation does not", D(E(10, "**Status**: `fixed` — landed.", body=f"Landed in `{L}`.")),
         10, repo, 1, ["E3 no-landing-commit"], []),
        ("P6c 7 characters", D(E(10, f"**Status**: `fixed` — `{landed[:7]}`.")), 10, repo, 0, ["PASS"], []),
        ("P6d 40 characters", D(E(10, f"**Status**: `fixed` — `{landed}`.")), 10, repo, 0, ["PASS"], []),
        ("P6e hash then subject in one span", D(E(10, f"**Status**: `fixed` — `{L} fix(x): y`.")), 10, repo, 0, ["PASS"], []),
        ("P6f 6 characters is not one", D(E(10, f"**Status**: `fixed` — `{landed[:6]}`.")), 10, repo, 1,
         ["E3 no-landing-commit"], []),
        ("P6g 41 characters is not one", D(E(10, f"**Status**: `fixed` — `{landed}a`.")), 10, repo, 1,
         ["E3 no-landing-commit"], []),
        ("P6h all-digit spans are not", D(E(10, "**Status**: `fixed` — CI run `42762269706` green.")), 10, repo, 1,
         ["E3 no-landing-commit", "all-digit"], ["E4"]),
        ("P6i all-digit spans beside a real one", D(E(10, f"**Status**: `fixed` — `{L}`, run `42762269706`, `20261110`.")),
         10, repo, 0, ["PASS", "1 commit citation(s)"], []),
        ("P6j labelled hex is not one", D(E(10, f"**Status**: `fixed` — `{L}`; body `sha256 9f8e7d6c5b4a`.")),
         10, repo, 0, ["PASS", "1 commit citation(s)"], []),
        ("P6k uppercase is not one", D(E(10, f"**Status**: `fixed` — `{L.upper()}`.")), 10, repo, 1,
         ["E3 no-landing-commit", "lowercase"], []),
        ("P6l hex after the backtick only", D(E(10, f"**Status**: `fixed` — see `x{L}`.")), 10, repo, 1,
         ["E3 no-landing-commit"], []),
        # ── P7 a citation must RESOLVE, to a LANDED commit, on terminal rows ──
        ("P7a unknown hash", D(E(10, "**Status**: `fixed` — `0000000a`.")), 10, repo, 1,
         ["E4 commit-not-landed", at(10), "0000000a", "not a commit"], []),
        ("P7b unlanded commit", D(E(10, f"**Status**: `fixed` — `{U}`.")), 10, repo, 1,
         ["E4 commit-not-landed", U, "NOT an ancestor of HEAD"], []),
        ("P7c one landed, one not", D(E(10, f"**Status**: `fixed` — `{L}` + `0000000a`.")), 10, repo, 1,
         ["E4 commit-not-landed", "0000000a"], [f"`{L}` is"]),
        ("P7d citations on an open row are not checked", D(E(10, "**Status**: `open` — suspect `0000000a`.")),
         10, repo, 0, ["PASS"], []),
        ("P7e a merge being concluded: MERGE_HEAD counts", D(E(10, f"**Status**: `fixed` — `{side3[:8]}`.")),
         10, merging, 0, ["PASS"], []),
        # ── P8 order, and the region is positional ──
        ("P8a duplicate id", D(E(10, ok_open), E(10, ok_open)), 10, repo, 1, ["E5 order", at(10), "follows #10"], []),
        ("P8b descending", D(E(11, ok_open), E(10, ok_open)), 10, repo, 1, ["E5 order", at(10), "follows #11"], []),
        ("P8c a gap is fine", D(E(10, ok_open), E(12, ok_open)), 10, repo, 0, ["PASS", "2 entries gated"], []),
        ("P8d an old id appended after the region", D(E(10, ok_open), E(11, ok_open), E(5, None)), 10, repo, 1,
         ["E5 order", at(5), "below the mark", "E1 no-status"], []),
        ("P8e an old-id § heading appended", D(E(10, ok_open), E(5, None, heading="### `#5` § Resolution — later")),
         10, repo, 1, ["E6 heading-format", "§ Resolution"], []),
        # ── P9 headings: nothing in the region may be invisible ──
        ("P9a unbackticked", D(E(10, ok_open), E(12, None, heading="### #12 — no backticks")), 10, repo, 1, ["E6 heading-format", "#12 — no"], []),
        ("P9b level four, new id", D(E(10, ok_open), E(12, None, heading="#### `#12` — level four")), 10, repo, 1,
         ["E6 heading-format", at(12), "level-4"], []),
        ("P9c level two, new id", D(E(10, ok_open), E(12, None, heading="## `#12` — level two")), 10, repo, 1,
         ["E6 heading-format", at(12), "level-2"], []),
        ("P9d an earlier id as a sub-section", D(E(10, ok_open), E(11, ok_open, body="#### `#10` revisited\n\nProse.")),
         10, repo, 0, ["PASS", "2 entries gated"], ["E6"]),
        ("P9e prose sub-heading mentioning a new id", D(E(10, ok_open, body="#### What `#12` did\n\nProse.")),
         10, repo, 0, ["PASS", "1 entry gated"], ["E6"]),
        ("P9f leading space", D(E(10, ok_open), E(12, None, heading=" ### `#12` — one space")), 10, repo, 1, ["E6 heading-format"], []),
        ("P9g decorated", D(E(10, ok_open), E(12, None, heading="### ✅ `#12` — done")), 10, repo, 1, ["E6 heading-format"], []),
        ("P9h `Bug #`", D(E(10, ok_open), E(12, None, heading="### Bug #12 — x")), 10, repo, 1, ["E6 heading-format"], []),
        ("P9i ASCII hyphen", D(E(10, ok_open), E(12, None, heading="### `#12` - hyphen")), 10, repo, 1, ["E6 heading-format"], []),
        ("P9j no title", D(E(10, ok_open), E(12, None, heading="### `#12`")), 10, repo, 1, ["E6 heading-format"], []),
        ("P9k suffixed id", D(E(10, ok_open), E(12, None, heading="### `#12b` — a sub-entry")), 10, repo, 1, ["E6 heading-format"], []),
        ("P9l prose ### inside the region", D(E(10, ok_open, body="### Notes\n\nProse.")), 10, repo, 1,
         ["E6 heading-format", "Notes"], []),
        ("P9m prose ### before the region", D(E(5, None, body="### Notes\n\nProse."), E(10, ok_open)), 10, repo, 0,
         ["PASS", "1 entry gated"], []),
        ("P9n below-mark malformed heading", D(E(5, None, heading="### #5 old style"), E(10, ok_open)), 10, repo, 0,
         ["PASS", "1 id heading(s) below the mark"], []),
        ("P9o new id at level four BEFORE the region", D(E(5, None, body="#### `#12` — early"), E(13, ok_open)), 10, repo, 1,
         ["E6 heading-format", at(12)], []),
        # ── P9 fences, CommonMark ──
        ("P9p fenced code is not a heading", D(E(10, ok_open, body="```bash\n# #12 a comment\n### `#13` — quoted\n```")),
         10, repo, 0, ["PASS", "1 entry gated"], ["E6", "E1"]),
        ("P9q four-backtick fence holding three", D(E(10, ok_open, body="````md\n```\n### `#13` — quoted\n```\n````"), E(11, None)),
         10, repo, 1, ["E1 no-status", at(11)], ["#13"]),
        ("P9r tilde fence", D(E(10, ok_open, body="~~~\n### `#13` — quoted\n~~~")), 10, repo, 0, ["PASS"], ["#13"]),
        ("P9s a fence line WITH info does not close", D(E(10, ok_open, body="```bash\nx\n```bash\n### `#13` — still quoted\n```")),
         10, repo, 0, ["PASS"], ["#13"]),
        ("P9t a longer run closes", D(E(10, ok_open, body="```\nx\n`````"), E(11, None)), 10, repo, 1,
         ["E1 no-status", at(11)], []),
        ("P9u four-space ``` is not a fence", D(E(10, ok_open, body="    ```text"), E(11, None)), 10, repo, 1,
         ["E1 no-status", at(11)], []),
        ("P9v ```inline``` is not a fence", D(E(10, ok_open, body="```inline``` prose"), E(11, None)), 10, repo, 1,
         ["E1 no-status", at(11)], []),
        # ── P10 cannot measure: every one is exit 2, never a pass ──
        ("P10a no vocabulary", "# Bug log\n\n- **Enforced from `#10`**\n\n" + E(10, ok_open), 10, repo, 2,
         ["CANNOT MEASURE", "Status values"], []),
        ("P10b `partial-fix` undeclared", D(E(10, ok_open), vocab=("fixed", "open")), 10, repo, 2,
         ["CANNOT MEASURE", "'partial-fix'"], []),
        ("P10c `fixed` undeclared", D(E(10, ok_open), vocab=("partial-fix", "open", "wontfix")), 10, repo, 2,
         ["CANNOT MEASURE", "'fixed'"], []),
        ("P10d mark points at nothing", D(E(10, ok_open), mark=100), 100, repo, 2, ["CANNOT MEASURE", "#100"], []),
        ("P10e shallow clone", D(E(10, fixed_ok)), 10, shallow, 2, ["CANNOT MEASURE", "SHALLOW", "--unshallow"], []),
        ("P10f not a repository", D(E(10, ok_open)), 10, notgit, 2, ["CANNOT MEASURE", "not a git repository"], []),
        ("P10g unclosed fence", D(E(10, ok_open, body="```bash\nnever closed"), E(11, None)), 10, repo, 2,
         ["CANNOT MEASURE", "never closed"], ["PASS"]),
        ("P10h header declares another mark", D(E(10, ok_open), mark=11), 10, repo, 2,
         ["CANNOT MEASURE", "Enforced from `#11`", "HIGH_WATER_MARK is #10"], []),
        ("P10i header declares no mark", D(E(10, ok_open), mark=None), 10, repo, 2,
         ["CANNOT MEASURE", "Enforced from"], []),
        ("P10j unreadable vocabulary item", D(E(10, ok_open), items="  - `fixed` — a.\n  - `partial-fix` — b.\n  - `re-emitted and gated` — c."),
         10, repo, 2, ["CANNOT MEASURE", "cannot read"], []),
        ("P10k wrapped vocabulary description", D(E(10, "**Status**: `wontfix` — x."),
                                                  items="  - `fixed` — landed, and the\n    description wraps.\n  - `partial-fix` — b.\n  - `open` — c.\n  - `wontfix` — d."),
         10, repo, 0, ["PASS", "wontfix 1"], []),
        ("P10l loose vocabulary list", D(E(10, "**Status**: `wontfix` — x."),
                                         items="  - `fixed` — a.\n\n  - `partial-fix` — b.\n\n  - `open` — c.\n\n  - `wontfix` — d."),
         10, repo, 0, ["PASS", "wontfix 1"], []),
    ]

    failures = 0
    names: List[str] = []

    def verdict(name: str, problems: List[str], rc: int, got: str) -> None:
        nonlocal failures
        names.append(name)
        if problems:
            failures += 1
            print(f"  FAIL {name}: {'; '.join(problems)}\n       output: {got.strip()[:400]}")
        else:
            print(f"  ok   {name} (rc {rc})")

    def one(path: Path, where: Path, mark: int, want_rc: int, must, mustnt, staged: bool = False):
        before = path.read_bytes() if path.exists() else None
        out, err = io.StringIO(), io.StringIO()
        problems = []
        try:
            rc = run(path, where, mark, staged=staged, out=out, err=err)
        except Exception as exc:  # a crash is a named failure, not a self-test traceback
            rc = -1
            problems.append(f"raised {type(exc).__name__}: {str(exc)[:200]}")
        got = out.getvalue() + err.getvalue()
        if rc != want_rc:
            problems.append(f"rc {rc}, wanted {want_rc}")
        problems += [f"missing {s!r}" for s in must if s not in got]
        problems += [f"unexpected {s!r}" for s in mustnt if s in got]
        if (path.read_bytes() if path.exists() else None) != before:  # bug-log #187: read-only
            problems.append("the gate modified the file it checks")
        return problems, rc, got

    path = tmp / "bug-log.md"
    for name, text, mark, where, want_rc, must, mustnt in probes:
        path.write_text(text, encoding="utf-8")
        verdict(name, *one(path, where, mark, want_rc, must, mustnt))

    # P10m-n: inputs a text fixture cannot express
    verdict("P10m missing file", *one(tmp / "absent.md", repo, 10, 2, ["CANNOT MEASURE", "does not exist"], []))
    path.write_bytes(D(E(10, ok_open)).encode("utf-8") + b"\n\xff\xfe stray bytes\n")
    verdict("P10n not UTF-8", *one(path, repo, 10, 2, ["CANNOT MEASURE", "not valid UTF-8"], ["Traceback"]))

    # P12 --staged reads the INDEX, not the working tree — both directions (review FP3/BYP-5/F3).
    srepo = tmp / "staged"
    srepo.mkdir()
    _fixture_repo(srepo)
    sfile = srepo / "docs" / "bug-log.md"
    sfile.parent.mkdir()
    good, bad = D(E(10, ok_open)), D(E(10, None))
    sfile.write_text(good, encoding="utf-8")
    _g(srepo, "add", "docs/bug-log.md")
    _g(srepo, "commit", "-qm", "log")
    sfile.write_text(bad, encoding="utf-8")
    _g(srepo, "add", "docs/bug-log.md")
    sfile.write_text(good, encoding="utf-8")  # violation staged, working tree clean
    verdict("P12a staged violation, clean tree: --staged fails", *one(sfile, srepo, 10, 1, ["(staged index)", "E1 no-status"], [], staged=True))
    verdict("P12b ...and the working-tree read passes", *one(sfile, srepo, 10, 0, ["(working tree)", "PASS"], []))
    _g(srepo, "add", "docs/bug-log.md")
    sfile.write_text(bad, encoding="utf-8")  # clean staged, unstaged draft violates
    verdict("P12c unstaged draft does not block --staged", *one(sfile, srepo, 10, 0, ["(staged index)", "PASS"], [], staged=True))
    verdict("P12d ...but the working-tree read sees it", *one(sfile, srepo, 10, 1, ["(working tree)", "E1 no-status"], []))
    idx2 = _g(srepo, "ls-files", "-s")
    one(sfile, srepo, 10, 0, [], [], staged=True)
    verdict("P12e --staged writes nothing to the index", [] if _g(srepo, "ls-files", "-s") == idx2 else ["the index changed"], 0, "")
    # The hook's index is whatever GIT_INDEX_FILE names (an index.lock under `commit -a`), so
    # --staged must read THAT index: stage a violation into an alternate index only.
    alt = tmp / "alt-index"
    alt.write_bytes((srepo / ".git" / "index").read_bytes())
    sfile.write_text(bad, encoding="utf-8")
    subprocess.run(["git", "-C", str(srepo), "add", "docs/bug-log.md"], check=True, capture_output=True,
                   env={**_clean_env(), "GIT_INDEX_FILE": str(alt)})
    sfile.write_text(good, encoding="utf-8")
    saved_idx = os.environ.get("GIT_INDEX_FILE")
    os.environ["GIT_INDEX_FILE"] = str(alt)
    try:
        verdict("P12g --staged reads the index GIT_INDEX_FILE names", *one(sfile, srepo, 10, 1, ["(staged index)", "E1 no-status"], [], staged=True))
    finally:
        if saved_idx is None:
            os.environ.pop("GIT_INDEX_FILE", None)
        else:
            os.environ["GIT_INDEX_FILE"] = saved_idx
    verdict("P12h ...and the default index without it", *one(sfile, srepo, 10, 0, ["(staged index)", "PASS"], [], staged=True))
    untracked = srepo / "docs" / "other.md"
    untracked.write_text(good, encoding="utf-8")
    verdict("P12f --staged on a file not in the index", *one(untracked, srepo, 10, 2, ["CANNOT MEASURE", "not in the index"], [], staged=True))

    # P11 the HOOK environment, recreated: git hands hooks an absolute GIT_INDEX_FILE (under
    # `commit -a`), GIT_CONFIG_PARAMETERS, and GIT_DIR/GIT_WORK_TREE in a linked worktree. Aim
    # them all at a REAL victim repository; the fixture must still build, the check must still
    # pass, and the victim's whole .git must be byte-identical afterwards.
    victim = tmp / "victim"
    victim.mkdir()
    _g(victim, "init", "-q")
    (victim / "victim.txt").write_text("v", encoding="utf-8")
    _g(victim, "add", "victim.txt")
    snap = _tree_snapshot(victim / ".git")
    hostile = {
        "GIT_INDEX_FILE": str(victim / ".git" / "index"),
        "GIT_DIR": str(victim / ".git"),
        "GIT_WORK_TREE": str(victim),
        # gpgsign/hooksPath are overridden by the fixture's own `-c` pins; core.bare is ignored
        # over a discovered work tree (both measured). core.sharedRepository=bogus makes
        # `git init` exit 128, so it is what makes a leak of THIS variable observable.
        "GIT_CONFIG_PARAMETERS": "'commit.gpgsign'='true' 'core.hooksPath'='/nonexistent' 'core.sharedRepository'='bogus'",
    }
    saved = {k: os.environ.get(k) for k in hostile}
    problems: List[str] = []
    try:
        os.environ.update(hostile)
        repo2 = tmp / "repo2"
        repo2.mkdir()
        landed2, _ = _fixture_repo(repo2)
        path.write_text(D(E(10, f"**Status**: `fixed` — `{landed2[:8]}`.")), encoding="utf-8")
        out, err = io.StringIO(), io.StringIO()
        rc = run(path, repo2, 10, out=out, err=err)
        if rc != 0:
            problems.append(f"rc {rc}, wanted 0: {(out.getvalue() + err.getvalue()).strip()[:300]}")
    except Exception as exc:  # a crash here is the failure being probed for
        problems.append(f"{type(exc).__name__}: {str(exc)[:300]}")
    finally:
        for k, v in saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
    if _tree_snapshot(victim / ".git") != snap:
        problems.append("the victim repository named by the inherited GIT_* was WRITTEN — the commit being made would be corrupted")
    verdict("P11 hook environment", problems, 0, "")

    # P13 main() under an ASCII-only stdout (review F7): the PASS line carries — – · and must
    # not raise UnicodeEncodeError. Runs the real entry point, so the fixture uses the real mark.
    p189 = tmp / "bl189.md"
    p189.write_text(D(E(HIGH_WATER_MARK, ok_open), mark=HIGH_WATER_MARK), encoding="utf-8")
    env = {**_clean_env(), "PYTHONIOENCODING": "ascii", "LC_ALL": "C", "LANG": "C"}
    r = subprocess.run([sys.executable, str(Path(__file__).resolve()), "--bug-log", str(p189), "--repo", str(repo)],
                       capture_output=True, env=env)
    verdict("P13 ASCII-only stdout", [] if r.returncode == 0 and b"PASS" in r.stdout else
            [f"rc {r.returncode}: {(r.stdout + r.stderr).decode('utf-8', 'replace').strip()[-300:]}"], r.returncode, "")

    # P14 the WHOLE self-test, as the hook runs it under `git commit -a`: a subprocess handed an
    # absolute GIT_INDEX_FILE aimed at a real victim index must pass, and the victim must be
    # byte-identical afterwards. The nested run skips P14 (no recursion).
    if os.environ.get("BUG_LOG_SELF_TEST_NESTED") != "1":
        victim14 = tmp / "victim14"
        victim14.mkdir()
        _g(victim14, "init", "-q")
        (victim14 / "v.txt").write_text("v", encoding="utf-8")
        _g(victim14, "add", "v.txt")
        snap14 = _tree_snapshot(victim14 / ".git")
        env14 = {**_clean_env(), "GIT_INDEX_FILE": str(victim14 / ".git" / "index"),
                 "BUG_LOG_SELF_TEST_NESTED": "1"}
        r14 = subprocess.run([sys.executable, str(Path(__file__).resolve()), "--self-test"],
                             capture_output=True, env=env14)
        problems14 = []
        if r14.returncode != 0:
            tail = [x for x in r14.stdout.decode("utf-8", "replace").splitlines() if "FAIL" in x][:3]
            problems14.append(f"nested self-test rc {r14.returncode}: {tail}")
        if _tree_snapshot(victim14 / ".git") != snap14:
            problems14.append("the hook's index was WRITTEN by the self-test")
        verdict("P14 the whole self-test under a hook's GIT_INDEX_FILE", problems14, r14.returncode, "")

    total = len(names)
    if failures:
        print(f"bug-log schema self-test: FAIL — {failures}/{total} probe(s) failed")
        return 1
    print(f"bug-log schema self-test: PASS — {total}/{total} probes")
    return 0


def main(argv: Optional[List[str]] = None) -> int:
    for stream in (sys.stdout, sys.stderr):  # review F7: a non-UTF-8 locale must not crash the PASS line
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")  # type: ignore[attr-defined]
        except Exception:
            pass
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("--bug-log", type=Path, default=BUG_LOG)
    ap.add_argument("--repo", type=Path, default=REPO_ROOT)
    ap.add_argument("--staged", action="store_true", help="read the bug-log from the index (the pre-commit hook)")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args(argv)
    if args.self_test:
        return self_test()
    return run(args.bug_log, args.repo, HIGH_WATER_MARK, staged=args.staged)


if __name__ == "__main__":
    sys.exit(main())
