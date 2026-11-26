"""Ansible evidence, format leg (docs/ai/design/ansible.md): the corpus files through ansible-lint --fix=yaml, Prettier and Sudsy (default and the yaml/prettier-compat preset), file by file.

Usage: evidence_ansible_format.py <work-dir> <sudsy> <ansible-lint> <prettier.cjs>
Copies the corpus files (scripts/evidence.py: Ansible kinds, 60 per project) to <work-dir>/<variant>/<project>/<path>, runs each tool on its copy and prints, for every pair, the files
that are byte-equal (all, deciding, held out), each tool's second-format change count (Sudsy only), and the first differing line of a sample.
"""
import shutil
import subprocess
import sys
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import evidence_ansible as mod  # noqa: E402

work, sudsy, lint, prettier = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4]
corpus = evidence.files(mod)
deciding, held = evidence.split(corpus, mod.SEED)
rels = [(p, f.relative_to(mod.CORPUS_ROOT / "ansible")) for p, _, f in corpus]
V = ["orig", "fix", "prettier", "sudsy", "sudsyp", "sudsya"]
if not (work / "orig").exists():
    for v in V:
        for p, rel in rels:
            (work / v / rel).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(mod.CORPUS_ROOT / "ansible" / rel, work / v / rel)
    (work / "sudsyp" / "sudsy.yaml").write_text("extends: [preset:yaml/prettier-compat]\n")
    # options that approximate ansible-lint's writer (ruamel: width 160, indented sequences, double quotes, `---`, one space before a comment)
    (work / "sudsya" / "sudsy.yaml").write_text("yaml:\n  format: {line_width: 160, sequence_indent: indented, quote_style: prefer_double, document_start: always, comment_spacing: 1, bracket_spacing: true}\n")
    byp = {}
    for p, rel in rels:
        byp.setdefault(p, []).append(str(Path(*rel.parts[1:])))
    run = lambda argv, cwd: subprocess.run(argv, cwd=cwd, capture_output=True, timeout=3000)  # noqa: E731
    with ThreadPoolExecutor(6) as ex:
        list(ex.map(lambda kv: run([lint, "--nocolor", "-q", "--fix=yaml", *kv[1]], work / "fix" / kv[0]), byp.items()))
    run(["node", prettier, "--no-config", "--no-editorconfig", "--embedded-language-formatting=off", "--write", "--log-level", "silent", "."], work / "prettier")
    run([sudsy, "format", "."], work / "sudsy")
    run([sudsy, "format", "."], work / "sudsyp")
    run([sudsy, "format", "."], work / "sudsya")


def read(v, rel):
    try:
        return (work / v / rel).read_text("utf-8")
    except (UnicodeDecodeError, FileNotFoundError):
        return None


half = {p: "deciding" if p in deciding else "held" for p, _ in rels}
pairs = [("sudsy", "fix"), ("sudsy", "prettier"), ("sudsyp", "prettier"), ("sudsyp", "fix"), ("sudsya", "fix"), ("prettier", "fix"), ("orig", "fix"), ("orig", "prettier"), ("orig", "sudsy"), ("orig", "sudsyp")]
print(f"{len(rels)} files, {len(set(p for p, _ in rels))} projects, {len(deciding)} deciding and {len(held)} held-out projects")
for a, b in pairs:
    n = Counter()
    for p, rel in rels:
        n["all"] += 1
        if read(a, rel) == read(b, rel):
            n["equal"] += 1
            n[half[p]] += 1
    print(f"{a} = {b}: {n['equal']} of {n['all']} ({100 * n['equal'] / n['all']:.1f}%), deciding {n['deciding']}, held {n['held']}")
# second format
for v in ("sudsy", "sudsyp", "sudsya"):
    before = {rel: read(v, rel) for _, rel in rels}
    subprocess.run([sudsy, "format", "."], cwd=work / v, capture_output=True)
    print(f"second format {v}: {sum(1 for _, rel in rels if read(v, rel) != before[rel])} files change")
# what the tools change in a file: first differing line classes of sudsyp vs prettier
c = Counter()
for p, rel in rels:
    a, b = read("sudsyp", rel), read("prettier", rel)
    if a != b and a is not None and b is not None:
        la, lb = a.split("\n"), b.split("\n")
        i = next((i for i in range(min(len(la), len(lb))) if la[i] != lb[i]), min(len(la), len(lb)))
        c[(la[i][:50] if i < len(la) else "<eof>", lb[i][:50] if i < len(lb) else "<eof>")] += 1
print("first difference sudsyp vs prettier, most common:")
for k, n in c.most_common(12):
    print(n, k)
