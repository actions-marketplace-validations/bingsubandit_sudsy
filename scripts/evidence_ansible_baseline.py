"""Ansible evidence, lint baseline: what Sudsy's YAML lint reports today against the ansible-lint target (docs/ai/design/ansible.md).

Usage: evidence_ansible_baseline.py <work-dir of evidence_lint.py> <sudsy lint --format json output of the copies in <fmt-dir>/lint>
A finding of ansible-lint is covered when the mapped Sudsy rule reports the same file and line. The target is the keep and taste rules of evidence_ansible_rules.py.
"""
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import evidence_ansible as mod  # noqa: E402

MAP = {  # ansible-lint rule -> Sudsy rule (yamllint equivalents with the ansible-lint yamllint configuration)
    "yaml[truthy]": "ambiguous-booleans", "yaml[line-length]": "max-line-length", "yaml[octal-values]": "no-octal-values",
    "yaml[trailing-spaces]": "no-trailing-whitespace", "yaml[comments]": "comment-space",
}
res = json.load(open(sys.argv[1] + "/lint.json"))
sud = defaultdict(set)
for d in json.load(open(sys.argv[2]))["diagnostics"]:
    sud[(d["path"], d["rule"])].add(d["range"]["start"]["line"])
corpus = evidence.files(mod)
deciding, _ = evidence.split(corpus, mod.SEED)
tot, cov, held_tot, held_cov = Counter(), Counter(), Counter(), Counter()
for r in res:
    path = r["file"].split("/", 1)[1]
    for f in r["findings"]:
        v = mod.RULE.get(f["code"], ("", "unjudged", ""))[1]
        if v not in ("keep", "taste"):
            continue
        tot[f["code"]] += 1
        hit = f["code"] in MAP and f["line"] in sud[(path, MAP[f["code"]])]
        cov[f["code"]] += hit
        if r["project"] not in deciding:
            held_tot["all"] += 1
            held_cov["all"] += hit
T, C = sum(tot.values()), sum(cov.values())
print(f"target {T} findings; covered today {C} ({100 * C / T:.1f}%); held out {held_cov['all']} of {held_tot['all']} ({100 * held_cov['all'] / held_tot['all']:.1f}%)")
for code, rule in MAP.items():
    extra = sum(len(v) for (p, rl), v in sud.items() if rl == rule)
    print(f"{code}\tansible-lint {tot[code]}\tcovered {cov[code]} ({100 * cov[code] / max(tot[code], 1):.1f}%)\t{rule} reports {extra} lines in all")
