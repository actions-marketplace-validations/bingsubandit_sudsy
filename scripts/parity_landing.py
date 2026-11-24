"""The landing check (docs/ai/development/parity.md#landing-check): a sample of corpus files that equal the reference formatter's output.

`PARITY_WRITE=1 scripts/parity.sh format <tool>` writes `tests/parity/<tool>/landing.txt` (`write`): the SAMPLE files of the pinned
corpus that are identical to the tool's output, in hash order of their path, each as `<16 hex digits of the output's hash> <path below
tests/conformance/corpus>`. `scripts/parity.sh landing [<tool>...]` (`check`) formats those files with the leg's own `sudsy.yaml` and
compares the hashes; it needs no reference tool. Usage: parity_landing.py <sudsy-binary> <target-dir> [<tool>...] | --fetch (the fetch scripts of the legs)
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
HEADER = "# Corpus files equal to the reference output when written: <hash of the output> <path below tests/conformance/corpus>. PARITY_WRITE=1 scripts/parity.sh format <tool> rewrites it.\n"


def normalize(config, text):
    """The text as the leg compares it (`ignore_trailing_blanks`: the tool keeps the blanks at the end of a line, Sudsy never does)."""
    return re.sub(r"[ \t]+$", "", text, flags=re.M) if config.get("ignore_trailing_blanks") else text


def digest(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]


def write(folder, tool, equal, corpus=CORPUS):
    """Writes the sample of `equal` (corpus path -> the output both sides agree on); a leg with no pinned corpus file writes nothing."""
    rel = {p.relative_to(corpus).as_posix(): t for p, t in equal.items() if p.is_relative_to(corpus)}
    chosen = sorted(sorted(rel, key=lambda r: hashlib.sha256(r.encode()).hexdigest())[:SAMPLE])
    target = folder / "landing.txt"
    if not chosen:
        print(f"landing sample {tool}: no identical corpus file, not written")
        return
    target.write_text(HEADER + "".join(f"{digest(rel[r])} {r}\n" for r in chosen), encoding="utf-8", newline="\n")
    print(f"landing sample {tool}: {len(chosen)} of {len(rel)} identical files written to {target}")


def read(folder):
    """[(hash, path)] of the sample file (none when it is absent)."""
    path = folder / "landing.txt"
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
    missing = [rel for _, rel in entries if not (corpus / rel).is_file()]
    if missing:
        fetch = " ".join(config.get("fetch", [])) or "its fetch script"
        why = f"{tool}: the corpus is absent ({len(missing)} of {len(entries)} sampled files, first {missing[0]}): run {fetch}"
        return ([why], None) if ci else ([], f"skipped, {why}")
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


def fetch_scripts():
    """The fetch scripts of the legs that have a sample, each once (CI runs them before the check; they are idempotent per pinned commit)."""
    legs = json.loads((ROOT / "scripts/parity_tools.json").read_text(encoding="utf-8"))["format"]
    names = sorted(p.parent.name for p in (ROOT / "tests/parity").glob("*/landing.txt"))
    return list(dict.fromkeys(f for n in names for f in legs[n].get("fetch", [])))


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    if sys.argv[1:] == ["--fetch"]:
        print("\n".join(fetch_scripts()))
        return 0
    import parity_format as pf

    sudsy, target, names = sys.argv[1], Path(sys.argv[2]).resolve(), sys.argv[3:]
    configs = pf.tools()["format"]
    legs = names or sorted(p.parent.name for p in (ROOT / "tests/parity").glob("*/landing.txt"))
    ci = bool(os.environ.get("CI"))
    problems = []

    def leg(tool):
        started = time.monotonic()
        found, note = check(tool, configs[tool], ROOT / "tests/parity" / tool, sudsy, target / "parity" / f"landing-{tool}", ci)
        return tool, found, note, time.monotonic() - started

    legs = sorted(legs, key=lambda t: "worker" not in configs[t].get("needs", []))  # the slow start of the C# worker first
    with ThreadPoolExecutor(os.cpu_count()) as pool:  # the legs are independent: each has its own folder
        for tool, found, note, seconds in pool.map(leg, legs):
            problems += found
            print(f"landing {tool}: {note or ('FAILED' if found else 'ok')} ({seconds:.1f}s)")
    for problem in problems:
        print(problem)
    print(f"landing: {len(legs)} leg(s), {len(problems)} failing")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
