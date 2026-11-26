"""Kubernetes evidence, plan per tool (docs/ai/design/kubernetes.md): for each linter its target (keep and taste), and the rules needed for 85% of it.

Usage: python3 scripts/evidence_kubernetes_plan.py <lint.json of evidence_lint.py>
Per tool: findings by verdict, then the kept rules in order of findings with the cumulative share of the target: raw, weighted by project (every project counts as much as any other), held-out projects.
The split of projects is the one of scripts/evidence.py (seed of evidence_kubernetes.py).
"""
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import evidence_kubernetes as mod  # noqa: E402

res = json.load(open(sys.argv[1]))
corpus = evidence.files(mod)
deciding, held = evidence.split(corpus, mod.SEED)
per_proj = Counter(r["project"] for r in res if r["findings"])
by_tool = defaultdict(lambda: defaultdict(list))
for r in res:
    for f in r["findings"]:
        by_tool[f["code"].split(":")[0]][f["code"]].append(r["project"])


def weighted(projects, total_of):
    """Mean over projects of (findings in the set / findings of the project in the target)."""
    c = Counter(projects)
    return sum(c[p] / total_of[p] for p in total_of) / len(total_of)


for tool, codes in by_tool.items():
    verdict = {c: mod.RULE.get(c, ("", "unjudged", ""))[1] for c in codes}
    print(f"\n## {tool}: {sum(len(v) for v in codes.values())} findings, {len({p for v in codes.values() for p in v})} projects")
    for v in ("keep", "taste", "env", "artifact", "bug"):
        n = sum(len(codes[c]) for c in codes if verdict[c] == v)
        if n:
            print(f"{v}: {sum(1 for c in codes if verdict[c] == v)} rules, {n} findings")
    kept = sorted(((c, p) for c, p in codes.items() if verdict[c] in ("keep", "taste")), key=lambda x: -len(x[1]))
    allp = [p for _, ps in kept for p in ps]
    total_proj = Counter(allp)
    print(f"target (keep and taste): {len(allp)} findings of {len(kept)} rules in {len(total_proj)} projects; held out {sum(1 for p in allp if p in held)}")
    cum, cum_held, cum_p = 0, 0, []
    reached = {}
    for i, (c, ps) in enumerate(kept, 1):
        cum += len(ps)
        cum_held += sum(1 for p in ps if p in held)
        cum_p += ps
        raw, w, h = cum / len(allp), weighted(cum_p, total_proj), cum_held / max(1, sum(1 for p in allp if p in held))
        for name, x in (("raw", raw), ("weighted", w), ("held-out", h)):
            reached.setdefault(name, i if x >= 0.85 else None)
            if reached[name] is None and x >= 0.85:
                reached[name] = i
        if i <= 14 or i == len(kept):
            print(f"{i}\t{c}\t{len(ps)}\t{100 * raw:.1f} / {100 * w:.1f} / {100 * h:.1f}")
    print("rules to reach 85% (raw / weighted / held out):", reached)
