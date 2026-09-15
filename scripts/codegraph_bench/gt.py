#!/usr/bin/env python3
"""Ground truth for measuring CodeGraph's Rust caller recall.

A small lexer: skips comments, (raw/byte) strings and char literals, and tracks
delimiter frames so every position knows (1) its innermost enclosing fn body and
(2) whether it is inside a macro invocation. Emits every fn definition and every
call-like occurrence `name(` / `name::<..>(` with its path prefix, whether it is
a method call, and the line of its enclosing fn.

usage: gt.py <root> <out.json> [include_prefix]
"""
import json, pathlib, re, sys, time

RAW = re.compile(r'b?r(#*)"')
KW = {"if", "while", "match", "for", "loop", "return", "fn", "let", "in", "as", "move",
      "async", "await", "unsafe", "impl", "trait", "struct", "enum", "mod", "use", "pub",
      "crate", "self", "super", "Self", "where", "type", "const", "static", "dyn", "ref",
      "mut", "else", "break", "continue", "extern", "Some", "Ok", "Err", "None", "Box"}

def idch(ch): return ch.isalnum() or ch == "_"

def scan(text):
    n = len(text); i = 0; line = 1
    frames, toks = [], []
    defs, calls, mods_inline, mods_file, uses = [], [], [], [], []
    pending_fn = None; pending_kw = None; expect_def = False; expect_mod = False; pending_mod = None
    def in_macro(): return any(f["macro"] for f in frames)
    def encl():
        for f in reversed(frames):
            if f["kind"] == "fn": return f["fn_line"]
        return 0
    def in_test():
        return any(f["kind"] == "mod" and re.fullmatch(r"(?:\w+_)?tests?", f["modname"]) for f in frames)
    def macro_name():
        for f in reversed(frames):
            if f["macro"]: return f["mname"]
        return ""
    def method_ctx():
        for f in reversed(frames):
            if f["kind"] in ("impl", "trait"): return True
            if f["kind"] == "fn": return False
        return False
    while i < n:
        if len(toks) > 256: del toks[:128]
        c = text[i]
        if c == "\n": line += 1; i += 1; continue
        if c in " \t\r\f\v": i += 1; continue
        if text.startswith("//", i):
            j = text.find("\n", i); i = n if j < 0 else j; continue
        if text.startswith("/*", i):
            d = 1; i += 2
            while i < n and d:
                if text.startswith("/*", i): d += 1; i += 2
                elif text.startswith("*/", i): d -= 1; i += 2
                else:
                    if text[i] == "\n": line += 1
                    i += 1
            continue
        if c == "#":
            k = i + 1
            if k < n and text[k] == "!": k += 1
            while k < n and text[k] in " \t": k += 1
            if k < n and text[k] == "[":
                d = 0; q = k
                while q < n:
                    ch = text[q]
                    if ch == '"':
                        q += 1
                        while q < n and text[q] != '"':
                            q += 2 if text[q] == "\\" else 1
                    elif ch == "[": d += 1
                    elif ch == "]":
                        d -= 1
                        if d == 0: break
                    q += 1
                line += text.count("\n", i, q + 1); i = q + 1
                toks.append(("p", "#attr", i, line)); continue
        m = RAW.match(text, i)
        if m and (i == 0 or not idch(text[i - 1])):
            close = '"' + m.group(1); j = text.find(close, m.end())
            j = n if j < 0 else j + len(close)
            line += text.count("\n", i, j); i = j; toks.append(("lit", "", i, line)); continue
        if c == '"' or (c == "b" and text.startswith('b"', i) and (i == 0 or not idch(text[i - 1]))):
            j = i + (2 if c == "b" else 1)
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            j = min(n, j + 1); line += text.count("\n", i, j); i = j
            toks.append(("lit", "", i, line)); continue
        if c == "'":
            if text.startswith("\\", i + 1):
                j = text.find("'", i + 3); i = n if j < 0 else j + 1
                toks.append(("lit", "", i, line)); continue
            if i + 2 < n and text[i + 2] == "'":
                i += 3; toks.append(("lit", "", i, line)); continue
            toks.append(("p", "'", i, line)); i += 1; continue
        if c.isdigit():
            j = i + 1
            while j < n and idch(text[j]): j += 1
            if j + 1 < n and text[j] == "." and text[j + 1].isdigit():
                j += 1
                while j < n and idch(text[j]): j += 1
            toks.append(("num", "", i, line)); i = j; continue
        if c.isalpha() or c == "_":
            j = i + 1
            while j < n and idch(text[j]): j += 1
            w = text[i:j]
            if expect_def:
                expect_def = False
                pending_fn = dict(name=w, line=line, depth=len(frames), is_method=method_ctx())
                toks.append(("id", w, i, line)); i = j; continue
            if expect_mod:
                expect_mod = False
                k = j
                while k < n and text[k] in " \t\r\n": k += 1
                (mods_inline if k < n and text[k] == "{" else mods_file).append(w)
                if k < n and text[k] == "{": pending_mod = (w, len(frames))
            if w == "fn":
                k = j
                while k < n and text[k] in " \t\r\n": k += 1
                if k < n and (text[k].isalpha() or text[k] == "_"): expect_def = True
            elif w in ("impl", "trait"):
                pending_kw = (w, len(frames))
            elif w == "mod":
                expect_mod = True
            elif w == "use" and not in_macro():
                k = text.find(";", j)
                if k > 0: uses.append(re.sub(r"\s+", " ", text[j:k]).strip())
            elif w not in KW:
                k = j
                while k < n and text[k] in " \t\r\n": k += 1
                call = k < n and text[k] == "("; tf = False
                if not call and text.startswith("::<", k):
                    d = 0; q = k + 2
                    while q < n:
                        if text[q] == "<": d += 1
                        elif text[q] == ">":
                            d -= 1
                            if d == 0: break
                        q += 1
                    q += 1
                    while q < n and text[q] in " \t\r\n": q += 1
                    call = q < n and text[q] == "("; tf = call
                if call:
                    prefix = []; t = len(toks) - 1
                    while t >= 1 and toks[t][1] == "::" and toks[t - 1][0] == "id":
                        prefix.insert(0, toks[t - 1][1]); t -= 2
                    calls.append(dict(name=w, line=line, in_macro=in_macro(), prefix=prefix,
                                      dot=bool(toks) and toks[-1][1] == ".",
                                      qother=(not prefix and bool(toks) and toks[-1][1] == "::"),
                                      encl=encl(), in_test=in_test(), macro_name=macro_name(), turbofish=tf))
            toks.append(("id", w, i, line)); i = j; continue
        if text.startswith("::", i): toks.append(("p", "::", i, line)); i += 2; continue
        if text.startswith("!=", i): toks.append(("p", "!=", i, line)); i += 2; continue
        if c in "([{":
            macro = (len(toks) >= 2 and toks[-1][1] == "!" and toks[-2][0] == "id") or \
                    (len(toks) >= 3 and toks[-1][0] == "id" and toks[-2][1] == "!" and toks[-3][1] == "macro_rules")
            kind, fl, modname = "other", 0, ""
            mname = (toks[-2][1] if toks[-1][1] == "!" else "macro_rules") if macro else ""
            if c == "{":
                if pending_fn and pending_fn["depth"] == len(frames):
                    kind, fl = "fn", pending_fn["line"]
                    defs.append(dict(name=pending_fn["name"], line=pending_fn["line"],
                                     is_method=pending_fn["is_method"], has_body=True))
                    pending_fn = None; pending_kw = None
                elif pending_kw and pending_kw[1] == len(frames):
                    kind = pending_kw[0]; pending_kw = None
                elif pending_mod and pending_mod[1] == len(frames):
                    kind = "mod"; modname = pending_mod[0]; pending_mod = None
            frames.append(dict(macro=macro, kind=kind, fn_line=fl, modname=modname, mname=mname))
            toks.append(("p", c, i, line)); i += 1; continue
        if c in ")]}":
            if frames: frames.pop()
            if pending_kw and pending_kw[1] > len(frames): pending_kw = None
            if pending_fn and pending_fn["depth"] > len(frames): pending_fn = None
            toks.append(("p", c, i, line)); i += 1; continue
        if c == ";":
            if pending_fn and pending_fn["depth"] == len(frames):
                defs.append(dict(name=pending_fn["name"], line=pending_fn["line"],
                                 is_method=pending_fn["is_method"], has_body=False))
                pending_fn = None
            if pending_kw and pending_kw[1] == len(frames): pending_kw = None
        toks.append(("p", c, i, line)); i += 1
    return defs, calls, mods_inline, mods_file, uses

def find_crates(root):
    out = {}
    for ct in root.rglob("Cargo.toml"):
        if "target" in ct.parts: continue
        s = ct.read_text(errors="replace")
        if "[package]" not in s: continue
        pkg = re.search(r'\[package\][^\[]*?\bname\s*=\s*"([^"]+)"', s, re.S)
        lib = re.search(r'\[lib\][^\[]*?\bname\s*=\s*"([^"]+)"', s, re.S)
        nm = lib.group(1) if lib else (pkg.group(1) if pkg else ct.parent.name)
        out[str(ct.parent.relative_to(root))] = nm.replace("-", "_")
    return out

def main():
    root = pathlib.Path(sys.argv[1]).resolve(); out = sys.argv[2]
    inc = sys.argv[3] if len(sys.argv) > 3 else ""
    crates = find_crates(root)
    def crate_of(rel):
        best = None
        for cd in crates:
            if rel == cd or rel.startswith(cd + "/"):
                if best is None or len(cd) > len(best): best = cd
        return best
    t0 = time.time()
    files = sorted(p for p in root.rglob("*.rs")
                   if "target" not in p.relative_to(root).parts
                   and str(p.relative_to(root)).startswith(inc))
    defs, calls, meta, modfiles, errors = [], [], {}, {}, []
    for f in files:
        rel = str(f.relative_to(root))
        try:
            d, c, mi, mf, us = scan(f.read_text(errors="replace"))
        except Exception as e:
            errors.append(f"{rel}: {e}"); continue
        for x in d: x["file"] = rel
        for x in c: x["file"] = rel
        defs += d; calls += c
        meta[rel] = dict(inline=mi, filemods=mf, uses=us, crate=crate_of(rel))
        stem = f.parent.name if f.name == "mod.rs" else f.stem
        modfiles.setdefault(crate_of(rel), set()).add(stem)
    json.dump(dict(crates=crates, defs=defs, calls=calls, meta=meta,
                   modfiles={str(k): sorted(v) for k, v in modfiles.items()}, errors=errors),
              open(out, "w"))
    print(f"scanned {len(files)} files in {time.time() - t0:.1f}s: {len(defs)} fn defs, "
          f"{len(calls)} call-like sites, {len(errors)} errors")

if __name__ == "__main__":
    main()
