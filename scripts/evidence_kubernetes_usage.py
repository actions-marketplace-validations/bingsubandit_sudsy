"""Usage evidence of the Kubernetes manifest lint tools (docs/ai/design/kubernetes.md): which tools public repositories run.

Counts GitHub code search hits (10 queries a minute), repository stars, Homebrew installs (30 days), PyPI downloads, VS Code Marketplace installs.
Usage: python3 -I scripts/evidence_kubernetes_usage.py   (needs `gh` logged in)"""
import json
import subprocess
import time
import urllib.request

TOOLS = ["kubeconform", "kube-linter", "kube-score", "kubeval", "kubescape", "polaris", "datree", "conftest", "kyverno", "trivy config", "checkov"]
QUERIES = []
for t in TOOLS:
    QUERIES.append((f"{t} in a workflow", f'path:.github/workflows "{t}"'))
    QUERIES.append((f"{t} in a pre-commit config", f'filename:.pre-commit-config.yaml "{t}"'))
QUERIES += [("kube-linter config .yaml", "filename:.kube-linter.yaml"), ("kube-linter config .yml", "filename:.kube-linter.yml"),
            ("kustomization.yaml files", "filename:kustomization.yaml"), ("workflows with ignore-missing-schemas", 'path:.github/workflows "ignore-missing-schemas"')]
REPOS = ["yannh/kubeconform", "stackrox/kube-linter", "zegl/kube-score", "instrumenta/kubeval", "kubescape/kubescape", "FairwindsOps/polaris", "datreeio/datree",
         "open-policy-agent/conftest", "kyverno/kyverno", "aquasecurity/trivy", "bridgecrewio/checkov", "kubernetes-sigs/kustomize"]


def gh(args):
    for _ in range(3):
        r = subprocess.run(["gh", "api", "-X", "GET", *args], capture_output=True, text=True)
        if r.returncode == 0:
            return r.stdout.strip()
        time.sleep(30)
    return None


def get(url):
    return json.load(urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "evidence"})))


for r in REPOS:
    print("repo", r, gh([f"repos/{r}", "--jq", "[.stargazers_count,.pushed_at[:10],.archived,.license.spdx_id]|@tsv"]), flush=True)
for label, q in QUERIES:
    print(f"{gh(['search/code', '-f', f'q={q}', '-f', 'per_page=1', '--jq', '.total_count'])}\t{label}\t{q}", flush=True)
    time.sleep(7)
for f in ["kubeconform", "kube-linter", "kube-score", "kubeval", "kubescape", "polaris", "conftest", "checkov", "kyverno"]:
    try:
        print("brew installs 30d", f, get(f"https://formulae.brew.sh/api/formula/{f}.json")["analytics"]["install"]["30d"][f])
    except Exception as e:
        print("brew", f, "none", e)
for p in ["checkov", "kubernetes-validate"]:
    try:
        print("pypi last month", p, get(f"https://pypistats.org/api/packages/{p}/recent")["data"]["last_month"])
    except Exception as e:
        print("pypi", p, "none", e)
body = json.dumps({"filters": [{"criteria": [{"filterType": 7, "value": v}]} for v in
                   ["ms-kubernetes-tools.vscode-kubernetes-tools", "redhat.vscode-yaml", "stackrox.kubelinter"]], "flags": 914}).encode()
req = urllib.request.Request("https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery", body,
                             {"Content-Type": "application/json", "Accept": "application/json;api-version=3.0-preview.1", "User-Agent": "evidence"})
try:
    for f in json.load(urllib.request.urlopen(req))["results"]:
        for e in f["extensions"]:
            st = {s["statisticName"]: s["value"] for s in e["statistics"]}
            print("marketplace", e["publisher"]["publisherName"] + "." + e["extensionName"], "installs", int(st.get("install", 0)), "updated", e["lastUpdated"][:10])
except Exception as e:
    print("marketplace failed", e)
