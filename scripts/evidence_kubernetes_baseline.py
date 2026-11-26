"""Kubernetes lint coverage (docs/ai/design/kubernetes.md): the share of the kubeconform and kube-linter findings that the mapped Sudsy rule reports on the same line.

Usage: evidence_kubernetes_baseline.py <lint.json of evidence_lint.py> <sudsy lint --format json output>
The Sudsy run is `sudsy lint --format json` over copies of the corpus (`<project>/<path>`) with `extends: [preset:yaml/kubernetes]`. The target is the keep and
taste findings of kubeconform and kube-linter (evidence_kubernetes_rules.py); the numbers are raw per tool and pooled, for the deciding and the held-out projects.
"""
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import evidence_kubernetes as mod  # noqa: E402

MAP = {
    "kubeconform:schema-missing": "kubernetes-schema-missing",
    "kube-linter:no-read-only-root-fs": "kubernetes-no-read-only-root-fs",
    "kube-linter:run-as-non-root": "kubernetes-run-as-non-root",
    "kube-linter:unset-memory-requirements": "kubernetes-unset-memory-requirements",
    "kube-linter:unset-cpu-requirements": "kubernetes-unset-cpu-requirements",
    "kube-linter:latest-tag": "kubernetes-latest-tag",
}
res = json.load(open(sys.argv[1]))
sud = defaultdict(set)
for d in json.load(open(sys.argv[2]))["diagnostics"]:
    sud[(d["path"], d["rule"])].add(d["range"]["start"]["line"])
deciding, _ = evidence.split([(r["project"], "", "") for r in res], mod.SEED)
tot, cov = Counter(), Counter()
for r in res:
    path = r["file"].split("/", 1)[1]
    half = "deciding" if r["project"] in deciding else "held"
    for f in r["findings"]:
        tool = f["code"].split(":")[0]
        if tool not in ("kubeconform", "kube-linter") or mod.RULE.get(f["code"], ("", "unjudged", ""))[1] not in ("keep", "taste"):
            continue
        hit = f["code"] in MAP and f["line"] in sud[(path, MAP[f["code"]])]
        for key in (f["code"], tool, "pooled"):
            for h in ("all", half):
                tot[key, h] += 1
                cov[key, h] += hit


def line(name, key):
    t = lambda h: tot[key, h]  # noqa: E731
    c = lambda h: cov[key, h]  # noqa: E731
    pct = lambda h: f"{100 * c(h) / max(t(h), 1):.1f}%"  # noqa: E731
    print(f"{name}\ttarget {t('all')}\tcovered {c('all')} ({pct('all')})\tdeciding {pct('deciding')}\theld out {c('held')} of {t('held')} ({pct('held')})")


for tool in ("kubeconform", "kube-linter", "pooled"):
    line(tool, tool)
for code in MAP:
    line(f"  {code}", code)
extra = Counter(rl for (_, rl), v in sud.items() for _ in v)
print("Sudsy lines reported in all:", {rl: n for rl, n in sorted(extra.items())})
