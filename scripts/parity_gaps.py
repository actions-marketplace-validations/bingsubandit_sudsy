"""Ranks the causes of a wrap-compare run that lists its gaps (`PARITY_GAPS=1 PARITY_LABEL=x scripts/parity.sh format <tool>` writes
`<target>/parity/wrap-x.json`, one record per differing gap of every differing file, scripts/parity_wrap.py `gaps`).

A cause is the signature of a gap: cause kind, kinds of the two tokens around it, parent kind, the blanks of Sudsy's output and of the
reference (`--by full`), or without the blanks (`--by kinds`, the default: the rows of a rule are by kind). A file becomes equal when
every cause of it is fixed, so the number that ranks a set is the files it makes equal, not the gaps it covers.

  parity_gaps.py RUN.json [--group G] [--by kinds|full|shape] [--top N] [--steps N] [--fix PAT]... [--assume PAT]... [--yaml]

  --top N      the N causes in most files, and the files whose only cause each is
  --steps N    greedy sets: each step adds the cause that finishes the most files (ties and zero: the cause that brings the most files
               nearest to equal); prints the equal files after each step
  --fix PAT    files that become equal when exactly the causes matching PAT are fixed (repeatable: one set); PAT is `field=a|b,field=c`
               with the fields c (cause), a, b (token kinds), p, g (parent, grandparent), o, t (the blanks of Sudsy, of the reference)
  --assume PAT counts the causes matching PAT as fixed before the steps and the `--fix` sets (the other causes of that kind not yet)
  --yaml       the steps as `sets:` lines for the C/C++ part of differences.yaml
  --classify   the `files:` lines of differences.yaml: per group the named gap (`gap_name`) of the first gap of each differing file, counted
"""
import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path

FIELDS = {"full": "cabpgot", "kinds": "cabp", "shape": "cab"}


def pattern(text):
    pairs = [p.split("=", 1) for p in text.split(",")]
    return [(k, set(v.split("|"))) for k, v in pairs]


def matches(rec, pat):
    return all(rec[k] in v for k, v in pat)


def load(path, group, by):
    files = json.loads(Path(path).read_text(encoding="utf-8"))["files"]
    out, sample = {}, {}
    for name, w in files.items():
        if w["ok"] or (group and w["group"] != group):
            continue
        recs = w.get("gaps") or [{"c": "other", "a": "", "b": "", "p": "", "g": "", "o": "", "t": "", "x": ",".join(w["causes"])}]
        if len(recs) >= 400:
            recs = recs + [{"c": "capped", "a": "", "b": "", "p": "", "g": "", "o": "", "t": "", "x": ""}]
        out[name] = recs
        for r in recs:
            sample.setdefault(sig(r, by), r["x"].strip())
    return out, sample


def sig(rec, by):
    return tuple(rec[k] for k in FIELDS[by])


def label(s, by):
    if by == "full":
        return f"{s[0]} {s[1]}|{s[2]} in {s[3]}/{s[4]}: {s[5]!r} -> {s[6]!r}"
    return f"{s[0]} {s[1]}|{s[2]}" + (f" in {s[3]}" if len(s) > 3 else "")


MARKS = {"Star": "pointer-align", "Amp": "reference-align", "AmpAmp": "reference-align"}
SPACING = [  # (kinds of either token, name): the first row with a kind of the two tokens wins
    ({"Less", "Greater", "Spaceship"}, "angle-space"),
    ({"Lbrace", "Rbrace"}, "braced-list-padding"),
    ({"Comma"}, "comma-space"),
    ({"Question", "Colon"}, "ternary-colon-space"),
    ({"Hash", "HashHash"}, "preprocessor-token-space"),
    ({"Equal", "PlusEq", "MinusEq", "StarEq", "SlashEq", "PipeEq", "AmpEq", "CaretEq"}, "assignment-space"),
]
OPERATORS = {"Plus", "Minus", "Slash", "Percent", "Pipe", "Caret", "EqEq", "BangEq", "LessLess", "GreaterGreater", "PipePipe"}


def gap_name(g):
    """The named gap of one record (the names of the `differences` of the tool's differences.yaml)."""
    c, a, b, p, o, t = (g[k] for k in "cabpot")
    if c == "tokens-reorder":
        return "sort-using-declarations"
    if c == "tokens":
        return "angle-brackets-merged-by-reference" if o == ">" else "token-change"
    if c == "format-off":
        return "format-off"
    if c == "spacing":
        for mark in (a, b):
            if mark in MARKS:
                return "binary-op-space" if p == "Binary" else MARKS[mark]
        if a == "Rpar" and b == "Lpar":
            return "call-after-paren-space"
        if a == "Rpar" and b not in ("Lbrace", "Rpar", "Semi", "Comma"):
            return "cast-space"
        for kinds, name in SPACING:
            if a in kinds or b in kinds:
                return name
        if b == "Lpar" and a in ("Ident", "Keyword"):
            return "space-before-paren"
        if a == "String" and b == "Ident":
            return "string-macro-space"
        if {a, b} & OPERATORS:
            return "binary-op-space"
        return "column-align" if len(o) > 3 or len(t) > 3 else "spacing-rare"
    if c == "comment-gap":
        return "trailing-comment-spacing" if "nl" not in o + t else "comment-line-indent-or-join"
    if c == "brace-placement":
        return "empty-body-braces" if (a, b) == ("Lbrace", "Rbrace") else "brace-own-line-in-reference" if "nl" in t else "brace-own-line-in-source"
    if c == "statement-join":
        return "statement-joined-by-reference" if "nl" in t else "statement-split-by-reference"
    if c == "blank-lines":
        return "blank-before-close-brace" if b == "Rbrace" else "blank-lines-other"
    if c == "indent":
        return "escaped-newline-align" if p == "Directive" else "continuation-indent-of-non-wrap-line"
    return "unclassified"


def classify(files):
    """group -> name -> files whose first gap has that name."""
    out = defaultdict(Counter)
    for n, recs in files.items():
        out[n.split("/")[0]][gap_name(recs[0])] += 1
    return out


def greedy(remaining, steps):
    """[(cause, files made equal)] by greedy steps over `remaining`: file -> set of causes still open."""
    order, done = [], 0
    for _ in range(steps):
        score, finish = Counter(), Counter()
        for f, s in remaining.items():
            if not s:
                continue
            for c in s:
                score[c] += 1.0 / (len(s) * len(s))
                finish[c] += len(s) == 1
        if not score:
            break
        best = max(score, key=lambda c: (finish[c], score[c]))
        before = sum(1 for s in remaining.values() if not s)
        for s in remaining.values():
            s.discard(best)
        done = sum(1 for s in remaining.values() if not s)
        order.append((best, done - before, done))
    return order


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--group")
    ap.add_argument("--by", choices=FIELDS, default="kinds")
    ap.add_argument("--top", type=int, default=0)
    ap.add_argument("--steps", type=int, default=0)
    ap.add_argument("--fix", action="append", default=[])
    ap.add_argument("--assume", action="append", default=[])
    ap.add_argument("--yaml", action="store_true")
    ap.add_argument("--classify", action="store_true")
    a = ap.parse_args()
    files, sample = load(a.run, a.group, a.by)
    assumed = [pattern(p) for p in a.assume]
    causes = {n: {sig(r, a.by) for r in recs if not any(matches(r, p) for p in assumed)} for n, recs in files.items()}
    print(f"{len(files)} differing files{f' of {a.group}' if a.group else ''}, {sum(1 for s in causes.values() if not s)} equal with the assumed causes fixed")
    if a.top:
        cover, solo = Counter(), Counter()
        for s in causes.values():
            cover.update(s)
            if len(s) == 1:
                solo.update(s)
        print("== causes by files (files whose only cause it is)")
        for s, n in cover.most_common(a.top):
            print(f"{n:5} ({solo[s]:4}) {label(s, a.by)}   e.g. {sample[s]!r}")
        print("files by number of causes:", ", ".join(f"{k}:{v}" for k, v in sorted(Counter(min(len(s), 10) for s in causes.values()).items())))
    for text in a.fix:
        pats = [pattern(p) for p in text.split(";")]
        left = {n: {s for s in c if not any(matches(dict(zip(FIELDS[a.by], s)), p) for p in pats)} for n, c in causes.items()}
        gain = [n for n, s in left.items() if not s and causes[n]]
        by_group = defaultdict(int)
        for n in gain:
            by_group[n.split("/")[0]] += 1
        print(f"== fix {text}: {len(gain)} files become equal ({', '.join(f'{g} {v}' for g, v in sorted(by_group.items()))})")
    if a.classify:
        for group, names in sorted(classify(files).items()):
            print(f"  {group}: " + ", ".join(f"{k}:{v}" for k, v in names.most_common()))
    if a.steps:
        order = greedy({n: set(c) for n, c in causes.items()}, a.steps)
        print("== greedy set: cause (files made equal by it, files equal after it)")
        for i, (s, new, done) in enumerate(order, 1):
            line = f"{i:3}. {label(s, a.by)}: +{new}, {done}"
            print(line + f"   e.g. {sample[s]!r}")
        if a.yaml:
            print("sets:")
            for i, (s, new, done) in enumerate(order, 1):
                print(f'  - "{label(s, a.by)}: +{new} files, {done} equal after it"')


if __name__ == "__main__":
    main()
