"""Ansible evidence: classify the first difference of two format variants of scripts/evidence_ansible_format.py.

Usage: evidence_ansible_causes.py <work-dir> <variant a> <variant b> [<n-samples>]
Prints files per cause (first differing line pair, classified by shape) and n sample pairs.
"""
import random
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

work, va, vb = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
n = int(sys.argv[4]) if len(sys.argv) > 4 else 0
cause_of = Counter()
samples = defaultdict(list)


def shape(a, b):
    if a is None or b is None:
        return "eof"
    if a.strip() == b.strip():
        return "indent only" if a.lstrip() == b.lstrip() else "whitespace"
    if a.strip().strip("\"'") == b.strip().strip("\"'") or re.sub(r"[\"']", "", a) == re.sub(r"[\"']", "", b):
        return "quotes"
    if a.replace(" ", "") == b.replace(" ", ""):
        return "inner spaces"
    if a.strip() == "" or b.strip() == "":
        return "blank line"
    if a.strip().startswith("#") or b.strip().startswith("#"):
        return "comment"
    if a.strip() == "---" or b.strip() == "---":
        return "document start"
    return "other"


for f in sorted((work / va).rglob("*")):
    if not f.is_file():
        continue
    rel = f.relative_to(work / va)
    try:
        a, b = f.read_text("utf-8"), (work / vb / rel).read_text("utf-8")
    except (UnicodeDecodeError, FileNotFoundError):
        continue
    if a == b:
        continue
    la, lb = a.split("\n"), b.split("\n")
    i = next((i for i in range(min(len(la), len(lb))) if la[i] != lb[i]), None)
    if i is None:
        c, pair = "trailing lines", (len(la), len(lb))
    else:
        c, pair = shape(la[i], lb[i]), (la[i][:90], lb[i][:90])
    cause_of[c] += 1
    samples[c].append((str(rel)[-50:], i, pair))
tot = sum(cause_of.values())
print(f"{va} vs {vb}: {tot} files differ")
for c, k in cause_of.most_common():
    print(f"{k}\t{c}")
    for s in random.Random(7).sample(samples[c], min(n, len(samples[c]))):
        print(f"    {s}")
