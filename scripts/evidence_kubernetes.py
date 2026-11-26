"""Facts of the Kubernetes evidence (docs/ai/design/kubernetes.md) for scripts/evidence.py and scripts/evidence_lint.py: corpus, split, how a manifest is recognised, the three linters.

Usage: evidence_kubernetes.py detect   (recognition statistics of the corpus)
A manifest is a YAML file with a top-level `apiVersion:` and `kind:` line in one document and no `{{` (Helm and other templates are left out).
Linters (env K8S_BIN: folder of the binaries, K8S_CACHE: kubeconform schema cache), default flags, one run per file; a finding has no line in kubeconform and
kube-linter, so it is placed on the `kind:` line of the document of the object (kind and name) it names. Codes: `kubeconform:<class of the message>`,
`kube-linter:<check>`, `kube-score:<check id>`.
"""
import json
import os
import re
import subprocess
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

CORPUS_ROOT = Path(__file__).resolve().parent.parent / "tests/conformance/corpus"
SEED = 20261204
CAP = 60
DIRS = [("kubernetes", re.compile(r"^.*\.ya?ml$"))]
PROJECT = {}
SKIP = re.compile(r"/(\.github|node_modules|vendor|\.git|charts|templates)/")
from evidence_kubernetes_rules import FAMILY, RULE  # noqa: E402,F401  (the lint verdicts)

BIN = Path(os.environ.get("K8S_BIN", "/scratch/slots/sudsy/7/k8s-eval/bin"))
CACHE = os.environ.get("K8S_CACHE", "/scratch/slots/sudsy/7/k8s-eval/schema-cache")
API = re.compile(r"^apiVersion:\s*['\"]?(?:[a-z0-9][a-z0-9.-]*/)?v\d+(?:alpha|beta)?\d*['\"]?\s*(?:#.*)?$", re.M)
KIND = re.compile(r"^kind:\s*['\"]?[A-Z]", re.M)


def is_manifest(text):
    return bool(API.search(text) and KIND.search(text)) and "{{" not in text


def KEEP(rel):
    try:
        f = CORPUS_ROOT / "kubernetes" / rel
        return f.stat().st_size < 2_000_000 and is_manifest(f.read_text("utf-8", "replace"))
    except OSError:
        return False


def docs(text):
    """[(start line, kind, name)] of the documents of a file (1-based line of the `kind:` line)."""
    out, start, lines = [], 0, text.split("\n")
    chunks, cur = [], []
    for i, ln in enumerate(lines):
        if ln.startswith("---"):
            chunks.append((start, cur))
            start, cur = i + 1, []
        else:
            cur.append((i, ln))
    chunks.append((start, cur))
    for start, cur in chunks:
        kind = name = None
        line = start + 1
        in_meta = False
        for i, ln in cur:
            m = re.match(r"^kind:\s*['\"]?([^\s'\"#]+)", ln)
            if m and kind is None:
                kind, line = m.group(1), i + 1
            if ln.startswith("metadata:"):
                in_meta = True
            elif in_meta and ln and not ln.startswith((" ", "\t", "#")):
                in_meta = False
            m = re.match(r"^\s+name:\s*['\"]?([^\s'\"#]+)", ln)
            if in_meta and m and name is None:
                name = m.group(1)
        if kind:
            out.append((line, kind, name))
    return out


def place(index, kind, name):
    for line, k, n in index:
        if k == kind and (name in (None, "") or n == name):
            return line
    for line, k, n in index:
        if k == kind:
            return line
    return index[0][0] if index else 1


KC_CLASS = [("could not find schema", "schema-missing"), ("unmarshalling", "parse-error"), ("Invalid type", "invalid-type"), ("missing propert", "required"),
            ("Additional property", "additional-property"), ("must be one of", "enum"), ("does not match", "pattern"), ("Must validate", "one-of"), ("Must be", "bound")]


def kc_code(msg):
    for k, c in KC_CLASS:
        if k in msg:
            return "kubeconform:" + c
    return "kubeconform:other"


def run_file(path, rel):
    text = path.read_text("utf-8", "replace")
    index = docs(text)
    out = []
    r = subprocess.run([BIN / "kubeconform", "-output", "json", "-cache", CACHE, str(path)], capture_output=True, timeout=300)
    try:
        for x in json.loads(r.stdout)["resources"]:
            msg = x.get("msg", "")
            out.append({"line": place(index, x.get("kind"), x.get("name")), "code": kc_code(msg), "name": kc_code(msg)[12:], "severity": x["status"], "message": msg[:300], "context": f"{x.get('kind')}/{x.get('name')}"})
    except (ValueError, KeyError):
        out.append({"line": 1, "code": "kubeconform:tool-failure", "name": "tool-failure", "severity": "", "message": r.stderr.decode("utf-8", "replace")[:200], "context": ""})
    r = subprocess.run([BIN / "kube-linter", "lint", "--format", "json", str(path)], capture_output=True, timeout=300)
    try:
        for x in json.loads(r.stdout).get("Reports") or []:
            o = x["Object"]["K8sObject"]
            out.append({"line": place(index, o["GroupVersionKind"]["Kind"], o["Name"]), "code": "kube-linter:" + x["Check"], "name": x["Check"], "severity": "",
                        "message": x["Diagnostic"]["Message"][:300], "context": f"{o['GroupVersionKind']['Kind']}/{o['Name']}"})
    except (ValueError, KeyError):
        out.append({"line": 1, "code": "kube-linter:tool-failure", "name": "tool-failure", "severity": "", "message": r.stderr.decode("utf-8", "replace")[:200], "context": ""})
    r = subprocess.run([BIN / "kube-score", "score", "-o", "json", str(path)], capture_output=True, timeout=300)
    try:
        for o in json.loads(r.stdout) or []:
            k = o["type_meta"]["kind"]
            for c in o["checks"]:
                if c["skipped"] or c["grade"] >= 10:
                    continue
                out.append({"line": place(index, k, o["object_meta"].get("name")), "code": "kube-score:" + c["check"]["id"], "name": c["check"]["id"],
                            "severity": str(c["grade"]), "message": "; ".join(x["summary"] for x in c.get("comments") or [])[:300], "context": f"{k}/{o['object_meta'].get('name')}"})
    except (ValueError, KeyError):
        out.append({"line": 1, "code": "kube-score:tool-failure", "name": "tool-failure", "severity": "", "message": r.stderr.decode("utf-8", "replace")[:200], "context": ""})
    return out


def run_all(work, corpus):
    """The three linters over every file, one process per file and tool."""
    def one(item):
        proj, name, f = item
        try:
            return {"file": name, "project": proj, "findings": run_file(f, name)}
        except subprocess.TimeoutExpired:
            return {"file": name, "project": proj, "failed": "timeout", "findings": []}

    with ThreadPoolExecutor(int(os.environ.get("JOBS", "8"))) as ex:
        return list(ex.map(one, corpus))


if __name__ == "__main__":
    root = CORPUS_ROOT / "kubernetes"
    per, helm, manifest, multi, kinds, apis, nodoc = Counter(), 0, 0, 0, Counter(), Counter(), 0
    for f in sorted(root.rglob("*")):
        if not f.is_file() or not re.match(r".*\.ya?ml$", f.name) or SKIP.search(str(f.relative_to(root))):
            continue
        t = f.read_text("utf-8", "replace")
        per["all"] += 1
        if "{{" in t:
            helm += 1
            continue
        if not is_manifest(t):
            continue
        manifest += 1
        per[f.relative_to(root).parts[0]] += 1
        d = docs(t)
        multi += len(d) > 1
        for _, k, _n in d:
            kinds[k] += 1
        for m in re.finditer(r"^apiVersion:\s*['\"]?([^\s'\"#]+)", t, re.M):
            apis[m.group(1)] += 1
    print(per["all"], "yaml files;", manifest, "manifests;", helm, "with {{;", multi, "multi-document")
    print("kinds:", kinds.most_common(25))
    print("apiVersions:", apis.most_common(25))
