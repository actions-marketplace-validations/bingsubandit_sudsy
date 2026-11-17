#!/usr/bin/env python3
"""Hand-written Rust per language and the shared share of the workspace.

  cost_languages.py [REV]              table for REV (default: the working tree)
  cost_languages.py --check BASE [HEAD]  fail when the shared share falls or a language crosses its
                                       budget between BASE and HEAD without a `Cost-exception:` line

Classes and the budget are data: scripts/cost-kinds.txt. Called by `scripts/cost.sh --languages`.
Exception line, in the message of any commit of BASE..HEAD:
  Cost-exception: shared-share: <reason>      (the share fell by more than TOLERANCE points)
  Cost-exception: lang-kotlin: <reason>       (a language crossed the budget)
  Language-only: lang-kotlin: <the fact>      (a language crate grew: each added line is a fact of that
                                               language that cannot be generic; anything a second
                                               language does or could do belongs in a shared crate)
"""
import os
import re
import subprocess
import sys

TOLERANCE = 0.1  # percentage points the shared share may fall without a line
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def git(*args, inp=None):
    return subprocess.run(["git", "-C", ROOT, *args], input=inp, capture_output=True, check=True).stdout


def load_kinds():
    kinds, budget = [], 3000
    for line in open(os.path.join(ROOT, "scripts/cost-kinds.txt"), encoding="utf-8"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        k, v = line.split(None, 1)
        if k == "budget":
            budget = int(v)
        else:
            kinds.append((k, re.compile(v)))
    return kinds, budget


def read_files(rev, pattern):
    """{path: bytes} of the files of `rev` (None: working tree) matching `pattern`."""
    if rev is None:
        names = git("ls-files", "-co", "--exclude-standard").decode().splitlines()
        names = [n for n in names if pattern.search(n) and os.path.isfile(os.path.join(ROOT, n))]
        return {n: open(os.path.join(ROOT, n), "rb").read() for n in names}
    names = [n for n in git("ls-tree", "-r", "--name-only", rev).decode().splitlines() if pattern.search(n)]
    out = git("cat-file", "--batch", inp="".join(f"{rev}:{n}\n" for n in names).encode())
    files, pos = {}, 0
    for n in names:
        eol = out.index(b"\n", pos)
        size = int(out[pos:eol].split()[2])
        files[n] = out[eol + 1 : eol + 1 + size]
        pos = eol + size + 2
    return files


def measure(rev):
    kinds, budget = load_kinds()
    files = read_files(rev, re.compile(r"^(crates/[^/]+/src/.*\.rs|grammars/[^/]+\.grammar)$"))
    langs, shared = {}, 0
    nodes = {}
    for path, data in files.items():
        if path.endswith(".grammar"):
            nodes[os.path.basename(path)[:-8]] = sum(1 for l in data.splitlines() if l.startswith(b"node "))
            continue
        rel = path[len("crates/"):]
        lines = data.count(b"\n")
        cls = next((k for k, rx in kinds if rx.search(rel)), "other")
        if cls == "ignore":
            continue
        if cls == "shared":
            shared += lines
            continue
        crate = rel.split("/")[0]
        if not crate.startswith("lang-"):
            continue
        langs.setdefault(crate, {"formatter": 0, "rules": 0, "other": 0})[cls] += lines
    return langs, shared, nodes, budget


def total(d):
    return d["formatter"] + d["rules"] + d["other"]


def share(langs, shared):
    lang = sum(total(d) for d in langs.values())
    return 100.0 * shared / (shared + lang) if shared + lang else 0.0


def table(rev):
    langs, shared, nodes, budget = measure(rev)
    print(f"{'language':<20}{'nodes':>6}{'formatter':>10}{'rules':>8}{'other':>8}{'total':>8}{'per node':>9}")
    for crate in sorted(langs, key=lambda c: -total(langs[c])):
        d, n = langs[crate], nodes.get(crate[5:])
        per = f"{total(d) / n:.0f}" if n else "-"
        print(f"{crate[5:]:<20}{n if n else '-':>6}{d['formatter']:>10}{d['rules']:>8}{d['other']:>8}{total(d):>8}{per:>9}")
    lang = sum(total(d) for d in langs.values())
    print(f"shared crates {shared}, lang-* crates {lang}, shared share {share(langs, shared):.1f}% (budget {budget} lines per language)")


def check(base, head):
    b_langs, b_shared, _, budget = measure(base)
    h_langs, h_shared, _, _ = measure(head)
    msgs = git("log", "--format=%B", f"{base}..{head}").decode()
    excused = set(m.group(1) for m in re.finditer(r"^Cost-exception:\s*([\w-]+):\s*\S", msgs, re.M))
    language_only = set(m.group(1) for m in re.finditer(r"^Language-only:\s*([\w-]+):\s*\S", msgs, re.M))
    problems = []
    b_share, h_share = share(b_langs, b_shared), share(h_langs, h_shared)
    if h_share < b_share - TOLERANCE and "shared-share" not in excused:
        problems.append(f"shared share fell from {b_share:.1f}% to {h_share:.1f}% (add `Cost-exception: shared-share: <reason>`)")
    for crate, d in sorted(h_langs.items()):
        was = total(b_langs[crate]) if crate in b_langs else 0
        if was <= budget < total(d) and crate not in excused:
            problems.append(f"{crate} passed its budget of {budget} lines ({was} -> {total(d)}; add `Cost-exception: {crate}: <reason>`)")
        if total(d) > was and crate not in language_only:
            problems.append(f"{crate} grew ({was} -> {total(d)} lines): code that can be generic goes to the shared crates; "
                            f"if every added line is a fact of this language only, add `Language-only: {crate}: <the fact>`")
    for p in problems:
        print("cost check:", p)
    if problems:
        sys.exit(1)
    print(f"cost check OK (shared share {b_share:.1f}% -> {h_share:.1f}%)")


if __name__ == "__main__":
    a = sys.argv[1:]
    if a and a[0] == "--check":
        check(a[1], a[2] if len(a) > 2 else "HEAD")
    else:
        table(a[0] if a else None)
