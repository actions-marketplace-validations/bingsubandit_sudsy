"""The landing check (docs/ai/development/parity.md#landing-check): a sample of corpus files that equal the reference formatter's output.

`PARITY_WRITE=1 scripts/parity.sh format <tool>` writes `tests/parity/<tool>/landing.txt` (`write`): the SAMPLE files of the pinned
corpus that are identical to the tool's output, in hash order of their path, each as `<16 hex digits of the output's hash> <path below
tests/conformance/corpus>`. `scripts/parity.sh landing [<tool>...]` (`check`) formats those files with the leg's own `sudsy.yaml` and
compares the hashes; it needs no reference tool. The lint counterpart: `COVERAGE_WRITE=1 scripts/parity.sh coverage <tool>` writes `tests/parity/<tool>/landing-lint.txt` (`write_lint`): up to LINT_FILES
corpus files, each as `<16 hex digits of the hash of Sudsy's sorted (rule, line) findings of the mapped rules> <path>`, chosen among the files where
the tool's findings that count are all covered and Sudsy reports nothing extra (LINT_PER_RULE files per rule with findings, about LINT_ZERO of the files
with none); with too few agreeing files the sample is filled with files in the state Sudsy reports today. `check_lint` runs one `sudsy lint` per leg
under the leg's preset and compares the hashes.
Usage: parity_landing.py <sudsy-binary> <target-dir> [<tool>...] | --fetch (the fetch scripts of the legs)
"""
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / "tests/conformance/corpus"
SAMPLE = 400  # files per leg: a fall of 0.5% of the files is seen with about 85% probability (1 - 0.995^400 = 87%)
CHUNK = 100  # files per Sudsy run (a Windows command line is short)
LISTED = 20  # files named in a failure
LINT_FILE = "landing-lint.txt"
LINT_FILES = 150  # files per lint leg
LINT_PER_RULE = 3  # files per mapped rule that has findings
LINT_ZERO = 0.3  # share of the files with no finding of a mapped rule
LINT_AGREE = 50  # fewer agreeing files than this and the sample is filled with files in the state recorded as it is
LINT_HEADER = "# Corpus files: <hash of Sudsy's sorted (rule, line) findings of the mapped rules> <path below tests/conformance/corpus>. COVERAGE_WRITE=1 scripts/parity.sh coverage <tool> rewrites it.\n"
HEADER = "# Corpus files equal to the reference output when written: <hash of the output> <path below tests/conformance/corpus>. PARITY_WRITE=1 scripts/parity.sh format <tool> rewrites it.\n"


def normalize(config, text):
    """The text as the leg compares it (`ignore_trailing_blanks`: the tool keeps the blanks at the end of a line, Sudsy never does)."""
    return re.sub(r"[ \t]+$", "", text, flags=re.M) if config.get("ignore_trailing_blanks") else text


def digest(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]


def write(folder, tool, equal, corpus=CORPUS):
    """Writes the sample of `equal` (corpus path -> the output both sides agree on); a leg with no pinned corpus file writes nothing."""
    rel = {p.relative_to(corpus).as_posix(): t for p, t in equal.items() if p.is_relative_to(corpus)}
    chosen = sorted(order(rel)[:SAMPLE])
    target = folder / "landing.txt"
    if not chosen:
        print(f"landing sample {tool}: no identical corpus file, not written")
        return
    target.write_text(HEADER + "".join(f"{digest(rel[r])} {r}\n" for r in chosen), encoding="utf-8", newline="\n")
    print(f"landing sample {tool}: {len(chosen)} of {len(rel)} identical files written to {target}")


def read(folder, name="landing.txt"):
    """[(hash, path)] of the sample file (none when it is absent)."""
    path = folder / name
    lines = path.read_text(encoding="utf-8").splitlines() if path.exists() else []
    return [tuple(line.split(" ", 1)) for line in lines if line and not line.startswith("#")]


def source_of(config, path):
    """(fixture source, the file's name as the run keys it) of a corpus file."""
    for source in config["fixtures"]:
        if "dir" not in source:
            continue
        base = ROOT / source["dir"]
        if not path.is_relative_to(base):
            continue
        rel = path.relative_to(base).as_posix()
        if rel.endswith(tuple(source["suffixes"])) and rel.startswith(source.get("only", "")) and not rel.startswith(source.get("skip", "\0")):
            return source, source.get("prefix", "") + rel
    return None, None


def check(tool, config, folder, sudsy, work, ci, corpus=CORPUS):
    """Formats the sampled files of a leg and compares their hashes. Returns (problems, note): the problems fail the gate; a note
    (the corpus is absent and this is not CI) says why the leg was skipped."""
    import parity_format as pf

    entries = read(folder)
    if not entries:
        return [f"{tool}: tests/parity/{tool}/landing.txt is empty or missing: PARITY_WRITE=1 scripts/parity.sh format {tool} writes it"], None
    gone, note = absent(tool, config, entries, corpus, ci)
    if gone or note:
        return gone, note
    pf.use_worker(config)
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    shutil.copy(folder / "sudsy.yaml", work / "sudsy.yaml")
    runs = {}  # the folder a file is formatted in -> its copies
    expected = {}  # copy -> (the hash, the corpus path)
    for want, rel in entries:
        source, name = source_of(config, corpus / rel)
        if source is None:
            return [f"{tool}: {rel} is in no fixture source of {tool}: PARITY_WRITE=1 scripts/parity.sh format {tool} rewrites the sample"], None
        style = source.get("style")
        here = work / f"style-{style}" if style else work
        if style and not (here / "sudsy.yaml").exists():
            here.mkdir()
            shutil.copy(folder / f"{style}.yaml", here / "sudsy.yaml")
        copy = pf.copy_path(config, source, here, name)
        copy.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(corpus / rel, copy)
        runs.setdefault(here, []).append(copy)
        expected[copy] = (want, rel)
    for here, copies in runs.items():
        for i in range(0, len(copies), CHUNK):  # a file Sudsy rejects fails its run but the others are formatted
            subprocess.run([sudsy, config.get("sudsy_command", "format"), *(c.relative_to(here).as_posix() for c in copies[i : i + CHUNK])], cwd=here, capture_output=True, check=False)
    changed = [rel for copy, (want, rel) in expected.items() if digest(normalize(config, copy.read_bytes().decode("utf-8"))) != want]
    if not changed:
        return [], None
    shown = "".join(f"\n  {rel}" for rel in changed[:LISTED]) + (f"\n  and {len(changed) - LISTED} more" if len(changed) > LISTED else "")
    return [
        f"{tool}: {len(changed)} of {len(entries)} sampled files no longer equal the reference output:{shown}\n"
        f"  A difference that is wanted is recorded by running the leg again: PARITY_WRITE=1 scripts/parity.sh format {tool} (it rewrites tests/parity/{tool}/landing.txt).\n"
        f"  The same list appears when a corpus file changed: the fetch scripts pin its commit."
    ], None


def order(paths):
    """The paths in hash order of the path: the deterministic order every sample is chosen in."""
    return sorted(paths, key=lambda r: hashlib.sha256(r.encode()).hexdigest())


def lint_state(diagnostics, name_of, aliases, mapped):
    """{file name: {(rule, line)}} of the mapped rules in the JSON diagnostics of `sudsy lint` (`aliases`: an id that reports as another)."""
    out = {}
    for d in diagnostics:
        rule = aliases.get(d["rule"], d["rule"])
        if rule in mapped:
            out.setdefault(name_of(d["path"]), set()).add((rule, d["range"]["start"]["line"]))
    return out


def lint_digest(found):
    return digest("".join(f"{rule} {line}\n" for rule, line in sorted(found or ())))


def select_lint(states, agree):
    """The sample of a lint leg: `states` (corpus path -> {(rule, line)}), `agree` the paths whose findings the tool and Sudsy agree on.
    Returns (paths, filled): `filled` is true when too few agree and the sample holds files in the state recorded as it is."""
    pool = order(p for p in states if p in agree)
    filled = len(pool) < LINT_AGREE
    if filled:
        pool += order(p for p in states if p not in agree)
    by_rule = {}
    for p in pool:
        for rule in {r for r, _ in states[p]}:
            by_rule.setdefault(rule, []).append(p)
    chosen = []
    for rule in sorted(by_rule, key=lambda r: (len(by_rule[r]), r)):  # rare rules first when there are more than fit
        chosen += [p for p in by_rule[rule] if p not in chosen][:LINT_PER_RULE]
    chosen = chosen[:LINT_FILES]
    zero = [p for p in pool if not states[p] and p not in chosen]
    chosen += zero[: max(0, round(LINT_FILES * LINT_ZERO) - sum(not states[p] for p in chosen))]
    chosen += [p for p in pool if p not in chosen][: LINT_FILES - len(chosen)]
    return sorted(chosen[:LINT_FILES]), filled


def write_lint(folder, tool, states, agree):
    """Writes `landing-lint.txt` of a lint leg from `states` and `agree` (see `select_lint`); a leg with no pinned corpus file writes nothing."""
    if not states:
        print(f"landing-lint sample {tool}: no pinned corpus file, not written")
        return
    chosen, filled = select_lint(states, agree)
    (folder / LINT_FILE).write_text(LINT_HEADER + "".join(f"{lint_digest(states[p])} {p}\n" for p in chosen), encoding="utf-8", newline="\n")
    note = f"filled up with files in the state recorded as it is: only {len(agree)} agree" if filled else f"all agree ({len(agree)} files do)"
    print(f"landing-lint sample {tool}: {len(chosen)} of {len(states)} files written, {note}")


def absent(tool, config, entries, corpus, ci):
    """([problem], note) when a sampled corpus file is missing (a problem in CI, else a skip note); ([], None) when all are there."""
    missing = [rel for _, rel in entries if not (corpus / rel).is_file()]
    if not missing:
        return [], None
    fetch = " ".join(config.get("fetch", [])) or "its fetch script"
    why = f"{tool}: the corpus is absent ({len(missing)} of {len(entries)} sampled files, first {missing[0]}): run {fetch}"
    return ([why], None) if ci else ([], f"skipped, {why}")


def lint_changed(tool, config, entries, sudsy, work, corpus=CORPUS):
    """(the paths of `entries` whose hash differs, None) after one `sudsy lint` on only those files of a lint leg under its preset; (None, the problem) when it cannot run."""
    import parity_lint as pl

    spec = config.get("coverage") or {}
    names = pl.work_names(tool, spec)
    mapped, aliases = pl.sudsy_rules(tool)
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    expected = {}
    for want, rel in entries:
        name = names.get(corpus / rel)
        if name is None:
            return None, f"{tool}: {rel} is in no corpus source of {tool}: COVERAGE_WRITE=1 scripts/parity.sh coverage {tool} rewrites the sample"
        (work / name).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(corpus / rel, work / name)
        expected[name] = (want, rel)
    (work / "sudsy.yaml").write_text(json.dumps(pl.sudsy_config(tool, spec)), encoding="utf-8")
    (work / ".gitignore").write_text("!*\n", encoding="utf-8")  # the work folder may sit in a checkout (CI: target/) whose .gitignore (`bin/`) would hide corpus files
    run = subprocess.run([sudsy, "lint", "--format", "json", "."], cwd=work, capture_output=True, text=True, encoding="utf-8", check=False)
    try:
        diagnostics = json.loads(run.stdout)["diagnostics"]
    except (json.JSONDecodeError, KeyError):
        return None, f"{tool}: `sudsy lint` gave no report (exit {run.returncode}): {run.stderr.strip()[:300]}"
    state = lint_state(diagnostics, pl.name_fn(spec, work), aliases, mapped)
    return [rel for name, (want, rel) in expected.items() if lint_digest(state.get(name)) != want], None


def prune_lint(folder, tool, config, sudsy, work):
    """After `write_lint`: drops the files whose findings differ when linted without the rest of the corpus (a rule that reads the folder's other files)."""
    entries = read(folder, LINT_FILE)
    dropped = 0
    for _ in range(5):  # a file dropped can change what a neighbour in the sample reports
        changed, problem = lint_changed(tool, config, entries, sudsy, work)
        if problem:
            sys.exit(problem)
        if not changed:
            break
        entries = [e for e in entries if e[1] not in changed]
        dropped += len(changed)
    if dropped:
        (folder / LINT_FILE).write_text(LINT_HEADER + "".join(f"{want} {rel}\n" for want, rel in entries), encoding="utf-8", newline="\n")
        print(f"landing-lint sample {tool}: {dropped} files dropped, their findings depend on the other files of the corpus")


def check_lint(tool, config, folder, sudsy, work, ci, corpus=CORPUS):
    """Runs `sudsy lint` once on the sampled files of a lint leg, under its preset, and compares the hashes. Returns (problems, note)."""
    entries = read(folder, LINT_FILE)
    if not entries:
        return [f"{tool}: tests/parity/{tool}/{LINT_FILE} is empty or missing: COVERAGE_WRITE=1 scripts/parity.sh coverage {tool} writes it"], None
    gone, note = absent(tool, config, entries, corpus, ci)
    if gone or note:
        return gone, note
    changed, problem = lint_changed(tool, config, entries, sudsy, work, corpus)
    if problem:
        return [problem], None
    if not changed:
        return [], None
    shown = "".join(f"\n  {rel}" for rel in changed[:LISTED]) + (f"\n  and {len(changed) - LISTED} more" if len(changed) > LISTED else "")
    return [
        f"{tool}: {len(changed)} of {len(entries)} sampled files: the findings of the mapped rules changed:{shown}\n"
        f"  A difference that is wanted is recorded by running the leg again: COVERAGE_WRITE=1 scripts/parity.sh coverage {tool} (it rewrites tests/parity/{tool}/{LINT_FILE}).\n"
        f"  The same list appears when a corpus file changed: the fetch scripts pin its commit."
    ], None


def fetch_scripts():
    """The fetch scripts of the legs that have a sample, each once (CI runs them before the check; they are idempotent per pinned commit)."""
    tools = json.loads((ROOT / "scripts/parity_tools.json").read_text(encoding="utf-8"))
    return list(dict.fromkeys(f for kind, file in (("format", "landing.txt"), ("lint", LINT_FILE)) for p in sorted((ROOT / "tests/parity").glob(f"*/{file}"))
                              for f in tools[kind][p.parent.name].get("fetch", [])))


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    if sys.argv[1:] == ["--fetch"]:
        print("\n".join(fetch_scripts()))
        return 0
    import parity_format as pf

    sudsy, target, names = sys.argv[1], Path(sys.argv[2]).resolve(), sys.argv[3:]
    tools = pf.tools()
    folders = names or sorted({p.parent.name for p in (ROOT / "tests/parity").glob("*/landing*.txt")})
    # a leg is (kind, tool): the format sample and the lint sample of a folder are two legs
    legs = [(kind, t) for t in folders for kind, file in (("format", "landing.txt"), ("lint", LINT_FILE)) if (ROOT / "tests/parity" / t / file).exists() or (names and kind == "format" and t in tools["format"])]
    ci = bool(os.environ.get("CI"))
    problems = []

    def leg(item):
        kind, tool = item
        started = time.monotonic()
        folder, work = ROOT / "tests/parity" / tool, target / "parity" / f"landing-{kind}-{tool}"
        found, note = (check if kind == "format" else check_lint)(tool, tools[kind][tool], folder, sudsy, work, ci)
        return item, found, note, time.monotonic() - started

    legs = sorted(legs, key=lambda l: "worker" not in tools[l[0]][l[1]].get("needs", []))  # the slow start of the C# worker first
    with ThreadPoolExecutor(os.cpu_count()) as pool:  # the legs are independent: each has its own folder
        for (kind, tool), found, note, seconds in pool.map(leg, legs):
            problems += found
            print(f"landing {kind} {tool}: {note or ('FAILED' if found else 'ok')} ({seconds:.1f}s)")
    for problem in problems:
        print(problem)
    print(f"landing: {len(legs)} leg(s), {len(problems)} failing")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
