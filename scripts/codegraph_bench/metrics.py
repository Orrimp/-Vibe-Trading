#!/usr/bin/env python3
"""CodeGraph Rust caller-recall census against the gt.py lexer ground truth.
usage: metrics.py <gt.json> <repo_root> LABEL=db [LABEL=db ...]"""
import json, random, re, sqlite3, sys
from collections import Counter, defaultdict

gt = json.load(open(sys.argv[1])); REPO = sys.argv[2]
DBS = [a.split("=", 1) for a in sys.argv[3:]]
WS = set(gt["crates"].values()); meta = gt["meta"]
modfiles = {k: set(v) for k, v in gt["modfiles"].items()}
by_name = defaultdict(list)
for d in gt["defs"]: by_name[d["name"]].append(d)
unique = {n: v[0] for n, v in by_name.items() if len(v) == 1}
STOP = set("""new default from into try_from try_into fmt clone eq ne cmp partial_cmp hash drop deref deref_mut
as_ref as_mut borrow next len is_empty get get_mut set insert remove push pop iter iter_mut into_iter map filter
collect unwrap expect ok err and_then or_else send recv try_recv lock read write flush close open run start stop
spawn join poll call apply update view load save parse to_string to_vec to_owned as_str as_bytes contains extend
append clear sort first last min max sum count find any all take skip zip chain rev split trim replace build
finish reset init value name id kind label key keys values entry tick elapsed now sleep subscribe emit handle
process execute validate check compute render draw size width height color style theme text title
floor ceil round abs sqrt powi powf ln exp trunc fract signum format_fields""".split())

def use_root(name, uses):
    pat = re.compile(r"\b%s\b" % re.escape(name))
    for u in uses:
        if pat.search(u):
            return re.split(r"::", u.lstrip("{ ").strip(), maxsplit=1)[0].strip()
    return None

def classify(c, t):
    m = meta[c["file"]]; p = c["prefix"]; is_m = t["is_method"]
    if c["qother"]: return None, "qualified <T as X>::f()"
    if c["dot"]:
        if not is_m: return None, "x.f() but target is a free fn"
        if c["name"] in STOP: return None, "x.f() with a generic method name"
        return "method  x.f()", None
    if not p:
        if is_m: return None, "bare f() but target is a method"
        r = use_root(c["name"], m["uses"])
        if r is None: return "bare f()  (same module / glob)", None
        if r in WS: return "bare f()  imported from other crate", None
        if r in ("crate", "self", "super"): return "bare f()  imported same-crate", None
        return None, "bare f() imported from an external crate"
    seg = p[0]
    if seg == "Self" or seg[:1].isupper():
        return ("assoc  Type::f()", None) if is_m else (None, "Type::f() but target is a free fn")
    if is_m: return None, "module path but target is a method"
    if seg in ("crate", "self", "super"): return "path  crate::/super::f()", None
    if seg in WS: return "path  other_crate::f()", None
    r = use_root(seg, m["uses"])
    if r is not None:
        if r in WS: return "path  m::f(), m from other crate", None
        if r in ("crate", "self", "super"): return "path  m::f(), m same-crate (use)", None
        return None, "path through an external crate"
    if seg in m["inline"]: return "path  m::f(), m inline module", None
    if seg in modfiles.get(str(m["crate"]), ()): return "path  m::f(), m same-crate file mod", None
    return None, "path through an external crate"

def is_test(c):
    return bool(c.get("in_test")) or any(s in c["file"] for s in ("/tests/", "/benches/", "/examples/"))

sites, excluded = [], Counter()
for c in gt["calls"]:
    t = unique.get(c["name"])
    if t is None: continue
    form, why = classify(c, t)
    if form is None: excluded[why] += 1; continue
    key = ("IN A MACRO (any form)" if c["in_macro"] else "module level (const/static init)" if c["encl"] == 0
           else "TURBOFISH f::<T>() (any form)" if c.get("turbofish") else form)
    sites.append(dict(c, form=form, key=key, test=is_test(c)))
pop = sorted({s["name"] for s in sites})
lenient = defaultdict(set)                     # any textual call of the name, keyed by enclosing fn
for c in gt["calls"]:
    if c["name"] in unique: lenient[c["name"]].add((c["file"], c["encl"]))
strict = defaultdict(set)                      # only ground-truth-valid sites
for s in sites: strict[s["name"]].add((s["file"], s["encl"]))

def evaluate(db):
    cur = sqlite3.connect(db).cursor()
    tnode = {}
    for n in pop:
        t = unique[n]; best = None
        for nid, fp, sl in cur.execute(
                "select id, file_path, start_line from nodes where name=? and kind in ('function','method')", (n,)):
            if fp == t["file"] and abs((sl or 0) - t["line"]) <= 3: best = nid; break
        tnode[n] = best
    callers = {n: ([] if nid is None else cur.execute(
        "select e.line, s.kind, s.file_path, s.start_line from edges e join nodes s on s.id=e.source "
        "where e.target=? and e.kind='calls'", (nid,)).fetchall()) for n, nid in tnode.items()}
    unres = defaultdict(set)
    cols = [r[1] for r in cur.execute("pragma table_info('unresolved_refs')")]
    if {"file_path", "line", "reference_name"} <= set(cols):
        for fp, ln, rn in cur.execute("select file_path, line, reference_name from unresolved_refs"):
            unres[(fp, ln)].add(rn or "")
    R = dict(per_key=defaultdict(lambda: [0, 0]), miss=[], mech=Counter(), macro_miss=Counter(),
             not_indexed=sum(1 for v in tnode.values() if v is None))
    for s in sites:
        keys = {(fp, sl) for (_, _, fp, sl) in callers[s["name"]]}
        k = s["key"] if tnode[s["name"]] is not None else "target fn itself not indexed"
        R["per_key"][k][1] += 1
        if s["encl"] == 0:
            hitm = any(fp == s["file"] and abs(ln - s["line"]) <= 1 for (ln, _, fp, _) in callers[s["name"]])
        else:
            hitm = (s["file"], s["encl"]) in keys
        if hitm:
            R["per_key"][k][0] += 1
        else:
            R["miss"].append(dict(s, cat=k))
            hit = any(rn.endswith(s["name"]) for d in (-1, 0, 1) for rn in unres.get((s["file"], s["line"] + d), ()))
            R["mech"][(k, "extracted, unresolved" if hit else "never extracted")] += 1
            if s["in_macro"]: R["macro_miss"][s.get("macro_name") or "?"] += 1
    cgc = Counter((n, fp, sl) for n in pop for (_, _, fp, sl) in callers[n])
    grp = defaultdict(list)
    for s in sites: grp[(s["name"], s["file"], s["encl"])].append(s)
    sf = Counter(); sfpt = Counter()
    for g, ss in grp.items():
        k = cgc.get(g, 0)
        for idx, s in enumerate(sorted(ss, key=lambda s: s["in_macro"])):
            if idx < k: sf[s["key"]] += 1; sfpt["test" if s["test"] else "production"] += 1
    R["site_found"] = sf; R["site_found_pt"] = sfpt
    R["dup"] = sum(1 for v in cgc.values() if v > 1)
    per = defaultdict(lambda: dict(gt=set(), prod=set()))
    for s in sites:
        per[s["name"]]["gt"].add((s["file"], s["encl"]))
        if not s["test"]: per[s["name"]]["prod"].add((s["file"], s["encl"]))
    z = np = af = 0
    for n, v in per.items():
        cg = {(fp, sl) for (_, kind, fp, sl) in callers[n] if kind in ("function", "method")}
        z += not cg; af += v["gt"] <= cg
        np += bool(v["prod"]) and not (cg & v["prod"])
    R["sym"] = dict(targets=len(per), zero=z, prod_t=sum(1 for v in per.values() if v["prod"]), no_prod=np, all=af)
    pairs = {(n, fp, sl) for n in pop for (_, kind, fp, sl) in callers[n] if kind in ("function", "method")}
    R["prec_len"] = sum(1 for (n, fp, sl) in pairs if (fp, sl) in lenient[n])
    R["prec_str"] = sum(1 for (n, fp, sl) in pairs if (fp, sl) in strict[n])
    R["pairs"] = len(pairs)
    R["unsup"] = sorted((n, fp, sl) for (n, fp, sl) in pairs if (fp, sl) not in strict[n])
    R["edge_line"] = {(n, fp, sl): ln for n in pop for (ln, _, fp, sl) in callers[n]}
    return R

res = {l: evaluate(db) for l, db in DBS}; L = [l for l, _ in DBS]; tot = len(sites)
def pc(a, b): return f"{100.0*a/b:5.1f}%  {a:>5}/{b:<5}" if b else f"{'-':>18}"
prod = sum(1 for s in sites if not s["test"])
print(f"GROUND TRUTH  {len(pop)} unique-name functions with >=1 call · {tot} call sites "
      f"({prod} production, {tot-prod} test) · {sum(excluded.values())} candidate sites excluded as not-ours/ambiguous")
print(f"\n{'call-site form':<40}" + "".join(f"{l:>20}" for l in L))
print(f"{'ALL CALL SITES (recall)':<40}" + "".join(f"{pc(sum(v[0] for v in res[l]['per_key'].values()), tot):>20}" for l in L))
for k in sorted(res[L[0]]["per_key"], key=lambda k: -res[L[0]]["per_key"][k][1]):
    print(f"  {k:<38}" + "".join(f"{pc(*res[l]['per_key'][k]):>20}" for l in L))
print(f"\n{'CALL-SITE recall (count-matched)':<40}" + "".join(f"{l:>20}" for l in L))
print(f"{'ALL CALL SITES':<40}" + "".join(f"{pc(sum(res[l]['site_found'].values()), tot):>20}" for l in L))
tp = {"production": sum(1 for s in sites if not s["test"]), "test": sum(1 for s in sites if s["test"])}
for part in ("production", "test"):
    print(f"  {'-- ' + part + ' code only':<38}" + "".join(f"{pc(res[l]['site_found_pt'][part], tp[part]):>20}" for l in L))
for k in sorted(res[L[0]]["per_key"], key=lambda k: -res[L[0]]["per_key"][k][1]):
    print(f"  {k:<38}" + "".join(f"{pc(res[l]['site_found'][k], res[l]['per_key'][k][1]):>20}" for l in L))
print("  (edges per caller-fn pair can repeat: " + ", ".join(f"{l}: {res[l]['dup']} pairs with >1 edge" for l in L) + ")")
print(f"\n{'per function (the dead-code question)':<40}" + "".join(f"{l:>20}" for l in L))
for lab, f in (("ALL callers found", lambda s: (s["all"], s["targets"])),
               ("reports ZERO callers (has >=1)", lambda s: (s["zero"], s["targets"])),
               ("misses EVERY production caller", lambda s: (s["no_prod"], s["prod_t"]))):
    print(f"  {lab:<38}" + "".join(f"{pc(*f(res[l]['sym'])):>20}" for l in L))
print(f"  {'target fn not indexed at all':<38}" + "".join(f"{res[l]['not_indexed']:>20}" for l in L))
print(f"\n{'precision (CodeGraph caller fns)':<40}" + "".join(f"{l:>20}" for l in L))
print(f"  {'textual call exists in that fn':<38}" + "".join(f"{pc(res[l]['prec_len'], res[l]['pairs']):>20}" for l in L))
print(f"  {'...and it is a ground-truth-valid call':<38}" + "".join(f"{pc(res[l]['prec_str'], res[l]['pairs']):>20}" for l in L))
for l in L:
    print(f"\nWHY MISSED  [{l}]")
    for (k, mech), n in sorted(res[l]["mech"].items(), key=lambda x: -x[1])[:12]:
        print(f"  {n:>5}  {mech:<22} {k}")
    print(f"  macros hiding calls: " + ", ".join(f"{m}!={n}" for m, n in res[l]["macro_miss"].most_common(10)))
print("\nEXCLUDED from ground truth (not ours / ambiguous):")
for why, n in excluded.most_common(): print(f"  {n:>6}  {why}")
lines = {}
def src(f, n):
    if f not in lines: lines[f] = open(f"{REPO}/{f}", errors="replace").read().split("\n")
    return lines[f][n - 1].strip()[:110] if 0 < n <= len(lines[f]) else ""
random.seed(5); r0 = res[L[0]]
print(f"\nAUDIT SAMPLE — random misses [{L[0]}] (are these real calls?)")
bycat = defaultdict(list)
for m in r0["miss"]: bycat[m["cat"]].append(m)
for cat, ms in sorted(bycat.items(), key=lambda x: -len(x[1])):
    for m in random.sample(ms, min(2, len(ms))):
        print(f"  [{cat[:24]:<24}] {m['name']:<28} {m['file']}:{m['line']}  {('('+m.get('macro_name','')+'!) ') if m['in_macro'] else ''}{src(m['file'], m['line'])}")
print(f"\nAUDIT SAMPLE — CodeGraph callers WITHOUT a valid ground-truth call [{L[0]}]: {len(r0['unsup'])} total")
for (n, fp, sl) in random.sample(r0["unsup"], min(10, len(r0["unsup"]))):
    ln = r0["edge_line"].get((n, fp, sl))
    print(f"  {n:<28} <- {fp}:{sl}  edge@{ln}: {src(fp, ln) if ln else ''}")
print("\nKNOWN CASES — the runtime.rs sites:")
for s in sites:
    if s["file"].endswith("agent/src/runtime.rs") and s["name"] in ("check_and_liquidate", "plan_open_short"):
        print(f"  {s['name']:<22} :{s['line']}  in_macro={s['in_macro']} macro={s.get('macro_name')!r}  prefix={s['prefix']}")
