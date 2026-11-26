"""Ansible evidence: share of the corpus and of the findings in test trees (collection integration targets, molecule), which many projects exclude from ansible-lint.

Usage: evidence_ansible_tests.py <work-dir of evidence_lint.py>
"""
import json
import re
import sys
from collections import Counter

T = re.compile(r"/(tests/integration|tests/unit|molecule|tests/sanity|\.molecule)/")
res = json.load(open(sys.argv[1] + "/lint.json"))
tf = [r for r in res if T.search("/" + r["file"])]
print(f"files {len(res)}, in test trees {len(tf)} ({100 * len(tf) / len(res):.1f}%), projects with test files {len({r['project'] for r in tf})}")
n = sum(len(r["findings"]) for r in res)
nt = sum(len(r["findings"]) for r in tf)
print(f"findings {n}, in test trees {nt} ({100 * nt / n:.1f}%)")
a, b = Counter(), Counter()
for r in res:
    (b if T.search("/" + r["file"]) else a).update(f["code"] for f in r["findings"])
for c, k in (a + b).most_common(12):
    print(f"{c}\toutside {a[c]}\ttests {b[c]}")
