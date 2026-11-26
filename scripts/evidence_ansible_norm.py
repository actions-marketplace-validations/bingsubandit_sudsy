"""Ansible evidence: how many files of two format variants are equal once one cause is set aside (cumulative, in the order given).

Usage: evidence_ansible_norm.py <work-dir> <variant a> <variant b>
Causes: truthy (a plain yes/no/on/off/True/False value written true/false), quotes (every quote character removed), comments (spaces before a comment and after its `#` removed),
doc-start (a leading `---` line removed), blank (blank lines removed), indent (leading white space removed).
Prints files equal (all, deciding, held out) after each cause. The split is the one of scripts/evidence_ansible.py.
"""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import evidence_ansible as mod  # noqa: E402

work, va, vb = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
corpus = evidence.files(mod)
deciding, _ = evidence.split(corpus, mod.SEED)
rels = [(p, f.relative_to(mod.CORPUS_ROOT / "ansible")) for p, _, f in corpus]
NORMS = [
    ("none", lambda t: t),
    ("truthy", lambda t: re.sub(r"(:|-) (['\"]?)(yes|Yes|YES|True|TRUE|on|On|ON)\2[ \t]*(#.*)?$", r"\1 true \4", re.sub(r"(:|-) (['\"]?)(no|No|NO|False|FALSE|off|Off|OFF)\2[ \t]*(#.*)?$", r"\1 false \4", t, flags=re.M), flags=re.M)),
    ("quotes", lambda t: re.sub(r"[\"']", "", t)),
    ("comments", lambda t: re.sub(r"[ \t]*#[ \t]*", " #", t)),
    ("doc-start", lambda t: re.sub(r"\A---\n", "", t)),
    ("blank", lambda t: re.sub(r"\n\s*\n", "\n", t).strip()),
    ("indent", lambda t: re.sub(r"^[ \t]+", "", t, flags=re.M)),
]


def read(v, rel):
    try:
        return (work / v / rel).read_text("utf-8")
    except (UnicodeDecodeError, FileNotFoundError):
        return None


active = []
for name, fn in NORMS:
    active.append(fn)

    def norm(t):
        for f in active:
            t = f(t)
        return t

    eq = [p for p, rel in rels if read(va, rel) is not None and read(vb, rel) is not None and norm(read(va, rel)) == norm(read(vb, rel))]
    d = sum(1 for p in eq if p in deciding)
    nd = sum(1 for p, _ in rels if p in deciding)
    print(f"after {name}: {len(eq)} of {len(rels)} ({100 * len(eq) / len(rels):.1f}%), deciding {100 * d / nd:.1f}%, held out {100 * (len(eq) - d) / (len(rels) - nd):.1f}%")
