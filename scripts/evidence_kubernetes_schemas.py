"""Kubernetes evidence, schema data (docs/ai/design/kubernetes.md): how much data the kubeconform default needs and how well a table decides it.

Usage: python3 scripts/evidence_kubernetes_schemas.py <work-dir> <lint.json of evidence_lint.py>   (needs `gh` logged in, PyYAML)
1. The file list of the kubeconform default schema folder (yannh/kubernetes-json-schema `master-standalone`) as a table of kind and group/version; how well "the object's
   kind and apiVersion are in the table" predicts the `schema-missing` findings of kubeconform over the corpus (precision, recall).
2. The same for the CRD catalog (datreeio/CRDs-catalog): how many of the missing kinds a catalog schema exists for (files and objects).
3. The size of the OpenAPI spec of Kubernetes (api/openapi-spec/swagger.json), and of a condensed key-shape table from it: for each definition its property names with
   a type tag, required list and enum values, as JSON, raw and gzipped.
"""
import gzip
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

import yaml

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import evidence_kubernetes as mod  # noqa: E402

work = Path(sys.argv[1])
work.mkdir(parents=True, exist_ok=True)


def gh(*a):
    return subprocess.run(["gh", "api", *a], capture_output=True, check=True).stdout


def tree(spec):
    p = work / (re.sub(r"\W", "_", spec) + ".json")
    if not p.exists():
        p.write_bytes(gh(f"repos/{spec.split(':', 1)[0]}/git/trees/{spec.split(':', 1)[1]}?recursive=1"))
    return json.loads(p.read_text())["tree"]


def gvk_name(api, kind):
    """kubeconform file name of the default location: kind, then the first label of the group (if any) and the version."""
    g, _, v = api.rpartition("/")
    return f"{kind}-{g.split('.')[0]}-{v}.json".lower() if g else f"{kind}-{v}.json".lower()


std = {e["path"] for e in tree("yannh/kubernetes-json-schema:master:master-standalone") if e["path"].endswith(".json")}
cat = tree("datreeio/CRDs-catalog:main")
catalog = {e["path"] for e in cat if e["path"].endswith(".json")}
print(f"default schema folder master-standalone: {len(std)} files, kinds without version suffix excluded; CRD catalog: {len(catalog)} schemas in {len({p.split('/')[0] for p in catalog})} groups")

corpus = evidence.files(mod)
lint = {r["file"]: r for r in json.load(open(sys.argv[2]))}
tp = fp = fn = tn = 0
missing, objs = Counter(), Counter()
in_cat = 0
for proj, name, f in corpus:
    try:
        docs = [d for d in yaml.safe_load_all(f.read_text("utf-8", "replace")) if isinstance(d, dict) and "kind" in d and "apiVersion" in d]
    except yaml.YAMLError:
        continue
    miss_kinds = {x["context"].split("/")[0] for x in lint[name]["findings"] if x["code"] == "kubeconform:schema-missing"}
    for d in docs:
        api, kind = str(d["apiVersion"]), str(d["kind"])
        objs[(api, kind)] += 1
        pred = gvk_name(api, kind) not in std
        act = kind in miss_kinds
        tp += pred and act
        fp += pred and not act
        fn += (not pred) and act
        tn += (not pred) and not act
        if pred:
            missing[(api, kind)] += 1
print(f"table 'GVK not in the default folder' against kubeconform schema-missing, per object: both {tp}, table only {fp}, kubeconform only {fn}, neither {tn}")
print(f"{len(objs)} distinct GVKs in the corpus, {len(missing)} not in the default folder ({sum(missing.values())} objects)")
hit = {k: n for k, n in missing.items() if f"{k[0].rpartition('/')[0]}/{k[1].lower()}_{k[0].rpartition('/')[2]}.json" in catalog}
print(f"of the missing GVKs the CRD catalog has {len(hit)} ({sum(hit.values())} of {sum(missing.values())} objects)")
print("most common missing without catalog schema:", [(k, n) for k, n in sorted(missing.items(), key=lambda x: -x[1]) if k not in hit][:12])

spec = work / "swagger.json"
if not spec.exists():
    spec.write_bytes(gh("-H", "Accept: application/vnd.github.raw", "repos/kubernetes/kubernetes/contents/api/openapi-spec/swagger.json"))
defs = json.loads(spec.read_text())["definitions"]


def tag(p):
    if "$ref" in p:
        return "@" + p["$ref"].rsplit(".", 1)[-1]
    t = p.get("type", "")
    if t == "array":
        return "[" + tag(p.get("items", {})) + "]"
    if t == "object" and "additionalProperties" in p:
        return "{" + tag(p["additionalProperties"]) + "}"
    return t[:1] if t else "?"


table = {n: {"r": d.get("required", []), "p": {k: tag(v) for k, v in d.get("properties", {}).items()}} for n, d in defs.items()}
raw = json.dumps(table, separators=(",", ":")).encode()
print(f"swagger.json {spec.stat().st_size} bytes, {len(defs)} definitions; condensed key-shape table (names, type tags, required) {len(raw)} bytes, {len(gzip.compress(raw))} gzipped; {sum(len(t['p']) for t in table.values())} properties")
