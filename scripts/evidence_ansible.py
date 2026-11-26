"""Facts of the Ansible evidence (docs/ai/design/ansible.md) for scripts/evidence.py and scripts/evidence_lint.py: corpus, split, how a file is recognised, the primary linter.

Usage (a python with PyYAML, e.g. the ansible-lint venv): evidence_ansible.py detect   (recognition statistics of the corpus)
The kinds of a path are the list ansible-lint 26.11 uses (`DEFAULT_KINDS` of ansiblelint/config.py), first match wins.
"""
import json
import os
import re
from collections import Counter
from pathlib import Path

CORPUS_ROOT = Path(__file__).resolve().parent.parent / "tests/conformance/corpus"
SEED = 20261203
CAP = 60
DIRS = [("ansible", re.compile(r"^.*\.ya?ml$"))]
PROJECT = {}
KLASSES = {}
from evidence_ansible_rules import FAMILY, RULE  # noqa: E402,F401  (the lint verdicts)
SKIP = re.compile(r"/(\.github|node_modules|changelogs|\.tox|\.git)/")

KINDS = [  # (kind, glob) as ansible-lint's DEFAULT_KINDS: the kinds that name a YAML file of Ansible, or a non-Ansible one
    ("jinja2", "**/*.j2"), ("jinja2", "**/*.j2.*"), ("text", "**/templates/**/*.*"),
    ("execution-environment", "**/execution-environment.yml"), ("ansible-navigator-config", "**/ansible-navigator.{yaml,yml}"),
    ("ansible-lint-config", "**/.ansible-lint.{yaml,yml}"), ("inventory", "**/inventory/**.{yaml,yml}"),
    ("requirements", "**/meta/requirements.{yaml,yml}"), ("galaxy", "**/galaxy.yml"), ("reno", "**/releasenotes/*/*.{yaml,yml}"),
    ("vars", "**/{host_vars,group_vars,vars,defaults}/**/*.{yaml,yml}"), ("tasks", "**/tasks/**/*.{yaml,yml}"),
    ("play-argspec", "**/*.meta.{yaml,yml}"), ("playbook", "**/playbooks/*.{yml,yaml}"), ("playbook", "**/*playbook*.{yml,yaml}"),
    ("handlers", "**/handlers/*.{yaml,yml}"), ("test-meta", "**/tests/integration/targets/*/meta/main.{yaml,yml}"),
    ("meta", "**/meta/main.{yaml,yml}"), ("meta-runtime", "**/meta/runtime.{yaml,yml}"), ("role-arg-spec", "**/meta/argument_specs.{yaml,yml}"),
    ("requirements", "**/molecule/*/{collections,requirements}.{yaml,yml}"), ("yaml", "**/molecule/*/{base,molecule}.{yaml,yml}"),
    ("requirements", "**/requirements.{yaml,yml}"), ("playbook", "**/molecule/*/*.{yaml,yml}"), ("changelog", "**/changelogs/changelog.{yaml,yml}"),
    ("yaml", "**/*.{yaml,yml}"),
]
ANSIBLE = {"playbook", "tasks", "handlers", "vars", "meta", "test-meta", "role-arg-spec", "meta-runtime"}  # kinds the Ansible rules read


def glob_re(g):
    out, i = "", 0
    while i < len(g):
        if g.startswith("**/", i):
            out += "(?:.*/)?"
            i += 3
        elif g.startswith("**", i):
            out += ".*"
            i += 2
        elif g[i] == "*":
            out += "[^/]*"
            i += 1
        elif g[i] == "{":
            j = g.index("}", i)
            out += "(?:" + "|".join(re.escape(x) for x in g[i + 1:j].split(",")) + ")"
            i = j + 1
        else:
            out += re.escape(g[i])
            i += 1
    return re.compile("^" + out + "$")


KINDS = [(k, glob_re(g)) for k, g in KINDS]


def path_kind(rel):
    """Kind of a path below the project root (posix string)."""
    for k, r in KINDS:
        if r.match(rel):
            return k
    return "yaml"


TASK_KEYS = {"name", "block", "import_tasks", "include_tasks", "include_role", "import_role", "when", "tags", "action", "local_action"}


def content_kind(text):
    """Kind by content: playbook (list of plays), mixed, tasks (list of mappings with a task key), list, mapping, other, empty, invalid."""
    import yaml
    try:
        d = yaml.safe_load(text)
    except Exception:
        return "invalid"
    if isinstance(d, list) and d and all(isinstance(x, dict) for x in d):
        if all("hosts" in x or "import_playbook" in x for x in d):
            return "playbook"
        if any("hosts" in x or "import_playbook" in x for x in d):
            return "mixed"
        return "tasks" if any(TASK_KEYS & x.keys() for x in d) else "list"
    if isinstance(d, dict):
        return "mapping"
    return "other" if d is not None else "empty"


def KEEP(rel):
    return path_kind("/".join(rel.parts[1:])) in ANSIBLE


def run_all(work, corpus):
    """ansible-lint once per project over its files (it needs the roles and includes around a file); cwd the project root."""
    import subprocess
    from concurrent.futures import ThreadPoolExecutor
    by = {}
    for proj, name, f in corpus:
        by.setdefault(proj, []).append((name, f))
    env = {**os.environ, "HOME": str(work / "home"), "NO_COLOR": "1"}
    (work / "home").mkdir(parents=True, exist_ok=True)

    def one(item):
        proj, items = item
        root = CORPUS_ROOT / "ansible" / proj
        rels = [str(f.relative_to(root)) for _, f in items]
        try:
            r = subprocess.run([*LINT, *rels], cwd=root, env=env, capture_output=True, timeout=3000, check=False)
        except subprocess.TimeoutExpired:
            return [{"file": n, "project": proj, "failed": "timeout", "findings": []} for n, _ in items]
        out = r.stdout.decode("utf-8", "replace")
        (work / f"{proj}.stderr").write_bytes(r.stderr)
        res = []
        for (n, f), rel in zip(items, rels):
            try:
                res.append({"file": n, "project": proj, "findings": findings(out, rel)})
            except (ValueError, KeyError) as e:
                res.append({"file": n, "project": proj, "failed": str(e), "findings": []})
        return res

    with ThreadPoolExecutor(int(os.environ.get("JOBS", "6"))) as ex:
        return [x for part in ex.map(one, by.items()) for x in part]


# ansible-lint 26.11.1 over the named files of a project (cwd the project root), default rule set, no profile, no config; JSON on stdout
LINT = [os.environ.get("ANSIBLE_LINT", "ansible-lint"), "--nocolor", "-q", "-f", "json", "--offline"]


def line_of(loc):
    b = (loc.get("positions") or loc.get("lines") or {}).get("begin", 1)
    return b.get("line", 1) if isinstance(b, dict) else b


def findings(out, path):
    try:
        data = json.loads(out)
    except ValueError:
        return []
    return [{"line": line_of(i["location"]), "code": i["check_name"], "name": i["check_name"],
             "severity": i.get("severity", ""), "message": i["description"], "context": ""} for i in data if i["location"]["path"] == path]


if __name__ == "__main__":
    root = Path(__file__).resolve().parent.parent / "tests/conformance/corpus/ansible"
    kinds, mism, per, miss = Counter(), Counter(), Counter(), Counter()
    for f in sorted(root.rglob("*")):
        if not f.is_file() or not re.match(r".*\.ya?ml$", f.name) or SKIP.search(str(f)):
            continue
        parts = f.relative_to(root).parts
        pk, ck = path_kind("/".join(parts[1:])), content_kind(f.read_text("utf-8", "replace"))
        kinds[pk] += 1
        mism[(pk if pk in ANSIBLE else "other-kind", ck)] += 1
        per[parts[0]] += 1
        if pk not in ANSIBLE and ck in ("playbook", "tasks", "mixed"):
            miss[parts[0]] += 1
    print(sum(kinds.values()), "files;", len(per), "projects")
    for k, n in kinds.most_common():
        print(f"{k}\t{n}")
    print("-- path kind x content kind")
    for (a, b), n in sorted(mism.items(), key=lambda x: -x[1])[:40]:
        print(f"{a}\t{b}\t{n}")
    print("-- Ansible by content, not by path kind: per project")
    for k, n in miss.most_common(8):
        print(f"{k}\t{n}")
