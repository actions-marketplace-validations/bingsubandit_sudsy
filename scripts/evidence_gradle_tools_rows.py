"""Rows of the decision table (evidence_gradle.py) over the output of a formatter run on WORK/corpus: per row, the share of the
occurrences (held-out projects) that has the form the table decided, for the corpus as written and for each formatted tree.
Usage: python3 -I scripts/evidence_gradle_tools_rows.py WORK <tree-folder>...   (trees: corpus-nlg, spotless/corpus)"""
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence
import evidence_gradle as mod

work, trees = Path(sys.argv[1]), sys.argv[2:]
corpus = evidence.files(mod)
deciding, held = evidence.split(corpus, mod.SEED)
C = evidence.CORPUS / "gradle"


def shares(root):
    occ = defaultdict(Counter)
    changed = 0
    for proj, _, f in corpus:
        if proj not in held:
            continue
        rel = f.relative_to(C)
        g = f if root is None else work / root / rel
        if not g.exists():
            continue
        t = evidence.read(g, True)
        if t is None:
            continue
        for row, vs in mod.variants(t).items():
            occ[row].update(vs)
    return occ


base = shares(None)
others = {t: shares(t) for t in trees}
print("| row | decided form | corpus as written | " + " | ".join(trees) + " |")
print("|---|---|---|" + "---|" * len(trees))
for row in mod.SUDSY:
    want = evidence.forms(mod, row)
    if not want:
        # undecided rows: the distribution of the corpus against that of each tree
        def top(o):
            tot = sum(o[row].values()) or 1
            return ", ".join(f"{v} {100 * n / tot:.0f}%" for v, n in o[row].most_common(3))
        print(f"| {row} | as written | {top(base)} | " + " | ".join(top(o) for o in others.values()) + " |")
        continue
    def pct(o):
        tot = sum(o[row].values())
        return f"{100 * sum(o[row][v] for v in want) / tot:.1f}% of {tot}" if tot else "-"
    print(f"| {row} | {'/'.join(sorted(want))} | {pct(base)} | " + " | ".join(pct(o) for o in others.values()) + " |")
