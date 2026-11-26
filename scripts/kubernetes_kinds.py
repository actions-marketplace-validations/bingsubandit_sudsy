"""The `apiVersion/Kind` pairs of the Kubernetes API (docs/ai/design/kubernetes.md), one per line: the default of the
`known` option of the rule `kubernetes-schema-missing`.

Usage: python3 -I scripts/kubernetes_kinds.py <swagger.json> [<yannh master-standalone file list json>]
`swagger.json` is `api/openapi-spec/swagger.json` of github.com/kubernetes/kubernetes (Apache-2.0): every definition's
`x-kubernetes-group-version-kind`. With the file list of yannh/kubernetes-json-schema `master-standalone` (the default
folder of kubeconform, `gh api repos/yannh/kubernetes-json-schema/git/trees/master:master-standalone?recursive=1`) only
the pairs that have a file there are kept (no `CustomResourceDefinition`, no `DeleteOptions`).
"""
import json
import sys


def pairs(swagger):
    out = set()
    for d in swagger["definitions"].values():
        for g in d.get("x-kubernetes-group-version-kind", []):
            version = f"{g['group']}/{g['version']}" if g["group"] else g["version"]
            out.add(f"{version}/{g['kind']}")
    return sorted(out)


def file_name(pair):
    """kubeconform's file of a pair: kind, first label of the group, version, lower case."""
    *version, kind = pair.split("/")
    group = version[0].split(".")[0] if len(version) == 2 else ""
    v = version[-1]
    return f"{kind}-{group}-{v}.json".lower() if group else f"{kind}-{v}.json".lower()


if __name__ == "__main__":
    table = pairs(json.load(open(sys.argv[1])))
    if len(sys.argv) > 2:
        tree = json.load(open(sys.argv[2]))["tree"]
        files = {e["path"].rsplit("/", 1)[-1] for e in tree if e["path"].endswith(".json")}
        kept = [p for p in table if file_name(p) in files]
        print(f"{len(table)} pairs in the spec, {len(kept)} with a file in the folder", file=sys.stderr)
        table = sorted(kept + ["v1/List"])  # kubeconform reads the items of a List by their own kinds
    print("\n".join(table))
