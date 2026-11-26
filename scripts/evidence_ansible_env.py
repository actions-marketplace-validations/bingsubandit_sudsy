"""Ansible evidence: what the installed collections change. Compares two cached ansible-lint runs (scripts/evidence_lint.py ansible <work-dir>).

Usage: evidence_ansible_env.py <work-dir without collections> <work-dir with collections>
Prints the findings of each run, the findings only one has, per rule.
"""
import json
import sys
from collections import Counter


def load(work):
    res = json.load(open(f"{work}/lint.json"))
    return {(r["file"], f["line"], f["code"]) for r in res for f in r["findings"]}


a, b = load(sys.argv[1]), load(sys.argv[2])
print(f"without {len(a)}, with {len(b)}, both {len(a & b)}, only without {len(a - b)}, only with {len(b - a)}")
d = Counter([f"-{k[2]}" for k in a - b] + [f"+{k[2]}" for k in b - a])
for k, n in d.most_common(30):
    print(f"{k}\t{n}")
