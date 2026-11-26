"""Kubernetes evidence, format leg (docs/ai/design/kubernetes.md): the manifests through Prettier and Sudsy (default and the yaml/prettier-compat preset), file by file.

Usage: evidence_kubernetes_format.py <work-dir> <sudsy> <prettier.cjs>
Copies the corpus files (scripts/evidence.py: manifests, 60 per project) to <work-dir>/<variant>/<project>/<path>, runs each tool on its copy and prints the files that are
byte-equal (all, deciding, held out) for each pair, the second-format change count of Sudsy, the time of Sudsy format and lint over the files, and the most common first differences.
"""
import shutil
import subprocess
import sys
import time
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import evidence_kubernetes as mod  # noqa: E402

work, sudsy, prettier = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
corpus = evidence.files(mod)
deciding, held = evidence.split(corpus, mod.SEED)
rels = [(p, f.relative_to(mod.CORPUS_ROOT / "kubernetes")) for p, _, f in corpus]
V = ["orig", "prettier", "sudsy", "sudsyp"]
t = {}
if not (work / "orig").exists():
    for v in V:
        for p, rel in rels:
            (work / v / rel).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(mod.CORPUS_ROOT / "kubernetes" / rel, work / v / rel)
    (work / "sudsyp" / "sudsy.yaml").write_text("extends: [preset:yaml/prettier-compat]\n")
    subprocess.run(["node", prettier, "--no-config", "--no-editorconfig", "--embedded-language-formatting=off", "--write", "--log-level", "silent", "."], cwd=work / "prettier", capture_output=True)
    for v in ("sudsy", "sudsyp"):
        a = time.time()
        subprocess.run([sudsy, "format", "."], cwd=work / v, capture_output=True)
        t[v] = time.time() - a


def read(v, rel):
    try:
        return (work / v / rel).read_text("utf-8")
    except (UnicodeDecodeError, FileNotFoundError):
        return None


half = {p: "deciding" if p in deciding else "held" for p, _ in rels}
print(f"{len(rels)} files, {len(set(p for p, _ in rels))} projects, {len(deciding)} deciding and {len(held)} held-out projects; format seconds {t}")
for a, b in [("sudsy", "prettier"), ("sudsyp", "prettier"), ("orig", "prettier"), ("orig", "sudsy"), ("orig", "sudsyp")]:
    n = Counter()
    for p, rel in rels:
        n["all"] += 1
        if read(a, rel) == read(b, rel):
            n["equal"] += 1
            n[half[p]] += 1
    print(f"{a} = {b}: {n['equal']} of {n['all']} ({100 * n['equal'] / n['all']:.1f}%), deciding {n['deciding']}, held {n['held']}")
for v in ("sudsy", "sudsyp"):
    before = {rel: read(v, rel) for _, rel in rels}
    subprocess.run([sudsy, "format", "."], cwd=work / v, capture_output=True)
    print(f"second format {v}: {sum(1 for _, rel in rels if read(v, rel) != before[rel])} files change")
a = time.time()
r = subprocess.run([sudsy, "lint", "."], cwd=work / "orig", capture_output=True)
print(f"sudsy lint over the files: {time.time() - a:.2f} s, {len(r.stdout.splitlines())} output lines")
c = Counter()
for p, rel in rels:
    x, y = read("sudsyp", rel), read("prettier", rel)
    if x != y and x is not None and y is not None:
        la, lb = x.split("\n"), y.split("\n")
        i = next((i for i in range(min(len(la), len(lb))) if la[i] != lb[i]), min(len(la), len(lb)))
        c[(la[i][:50] if i < len(la) else "<eof>", lb[i][:50] if i < len(lb) else "<eof>")] += 1
print("first difference sudsyp vs prettier, most common:")
for k, n in c.most_common(12):
    print(n, k)
