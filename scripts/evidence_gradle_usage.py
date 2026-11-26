"""Usage evidence of the Gradle legs (docs/ai/design/gradle.md): which format and lint tools Gradle projects run.

Counts GitHub code search hits (10 queries a minute), npm and PyPI-style download numbers and the VS Code Marketplace installs.
Usage: python3 -I scripts/evidence_gradle_usage.py   (needs `gh` logged in)"""
import json
import subprocess
import time
import urllib.request

QUERIES = [
    ("build.gradle files", 'filename:build.gradle'),
    ("settings.gradle files", 'filename:settings.gradle'),
    ("Spotless plugin in a build.gradle", 'filename:build.gradle "com.diffplug.spotless"'),
    ("Spotless with a groovyGradle block", 'filename:build.gradle groovyGradle'),
    ("Spotless with greclipse", 'filename:build.gradle greclipse'),
    ("Spotless in a build.gradle.kts", 'filename:build.gradle.kts "com.diffplug.spotless"'),
    ("npm-groovy-lint in any workflow", 'path:.github/workflows npm-groovy-lint'),
    ("npm-groovy-lint in a pre-commit config", 'filename:.pre-commit-config.yaml groovy-lint'),
    (".groovylintrc.json files", 'filename:.groovylintrc.json'),
    ("npm-groovy-lint in a package.json", 'filename:package.json npm-groovy-lint'),
    ("CodeNarc plugin in a build.gradle", 'filename:build.gradle codenarc'),
    ("Nebula gradle-lint plugin", 'filename:build.gradle "nebula.lint"'),
    ("Android lint block (lint { ... } in build.gradle)", 'filename:build.gradle lintOptions'),
    ("dependency-analysis plugin", 'filename:build.gradle "com.autonomousapps.dependency-analysis"'),
    ("versions plugin (ben-manes)", 'filename:build.gradle "com.github.ben-manes.versions"'),
    ("MegaLinter groovy flavour setting", 'filename:.mega-linter.yml GROOVY'),
    ("editorconfig for gradle files", 'filename:.editorconfig "[*.gradle]"'),
]


def gh_count(q):
    for _ in range(3):
        r = subprocess.run(["gh", "api", "-X", "GET", "search/code", "-f", f"q={q}", "-f", "per_page=1", "--jq", ".total_count"], capture_output=True, text=True)
        if r.returncode == 0:
            return int(r.stdout.strip())
        time.sleep(30)
    return None


def get(url):
    return json.load(urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "evidence"})))


for label, q in QUERIES:
    print(f"{gh_count(q)}\t{label}\t{q}", flush=True)
    time.sleep(7)

for pkg in ["npm-groovy-lint", "gradle-lint", "prettier-plugin-groovy", "prettier-plugin-jenkinsfile"]:
    try:
        print("npm last month", pkg, get(f"https://api.npmjs.org/downloads/point/last-month/{pkg}")["downloads"])
    except Exception as e:
        print("npm", pkg, "none", e)
body = json.dumps({"filters": [{"criteria": [{"filterType": 7, "value": v}]} for v in
                   ["NicolasVuillamy.vscode-groovy-lint", "vscjava.vscode-gradle", "richardwillis.vscode-spotless-gradle", "naco-siren.gradle-language"]],
                   "flags": 914}).encode()
req = urllib.request.Request("https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery", body,
                             {"Content-Type": "application/json", "Accept": "application/json;api-version=3.0-preview.1", "User-Agent": "evidence"})
try:
    for f in json.load(urllib.request.urlopen(req))["results"]:
        for e in f["extensions"]:
            st = {s["statisticName"]: s["value"] for s in e["statistics"]}
            print("marketplace", e["publisher"]["publisherName"] + "." + e["extensionName"], "installs", int(st.get("install", 0)), "updated", e["lastUpdated"][:10])
except Exception as e:
    print("marketplace failed", e)
