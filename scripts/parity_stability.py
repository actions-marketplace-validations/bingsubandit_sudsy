"""Stability of `sudsy format` on a corpus, for a language with no reference formatter (docs/ai/design/parity-target.md, stability).

An entry of the `stability` section of parity_tools.json is a ratchet group of `tests/parity/stability/differences.yaml`: `fetch` (scripts), `corpus` (as the
coverage mode: `dir` below the repo root, `glob` patterns, `skip`), `as` (the name a file gets in its own work folder; the corpus name when absent) and `place`
(the folder inside the work folder; the language is found by it for GitHub Actions). A file is stable when `sudsy format` parses it (no error), its safety
check passes (lossless, meaning preserved), a second format changes nothing and the engine laid it out instead of copying most of it through
(`N copied through` in the summary line). The run prints the files that are not, and fails below `ratchet` (the least stable files) or when `target-files`
(the files of the corpus: the denominator of the README table) is stale.
"""
import re
import shutil
import subprocess
import sys
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import yaml

import parity_coverage
import parity_format

ROOT = Path(__file__).resolve().parent.parent


def unstable(sudsy, file):
    """Why `file` (alone in its folder) is not stable, or None."""
    run = subprocess.run([sudsy, "format", file.name], cwd=file.parent, capture_output=True, text=True, encoding="utf-8", check=False)
    if run.returncode:
        return "refused by the safety check" if "internal error" in run.stderr else "does not parse or format"
    if re.search(r", [1-9]\d* copied through", run.stderr):
        return "copied through"
    once = file.read_bytes()
    again = subprocess.run([sudsy, "format", file.name], cwd=file.parent, capture_output=True, text=True, encoding="utf-8", check=False)
    return None if again.returncode == 0 and file.read_bytes() == once else "changes on a second format"


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    group, sudsy, target = sys.argv[1:4]
    spec = parity_format.tools()["stability"][group]
    for script in spec.get("fetch", []):
        parity_format.run(["bash", script], ROOT)
    filters = Counter()
    files = parity_coverage.corpus_files(spec, filters)
    work = Path(target).resolve() / "parity" / f"stability-{group}"
    shutil.rmtree(work, ignore_errors=True)
    copies = []
    for i, (name, source) in enumerate(files):
        copy = work / str(i) / spec.get("place", "") / spec.get("as", source.name)
        copy.parent.mkdir(parents=True)
        shutil.copy(source, copy)
        copies.append((name, copy))
    with ThreadPoolExecutor() as pool:
        reasons = list(pool.map(lambda c: unstable(sudsy, c[1]), copies))
    for (name, _), why in zip(copies, reasons):
        if why:
            print(f"{name}: {why}")
    known = yaml.safe_load((ROOT / "tests/parity/stability/differences.yaml").read_text(encoding="utf-8")) or {}
    stable, total = sum(why is None for why in reasons), len(files)
    floor, recorded = (known.get("ratchet") or {}).get(group, 0), (known.get("target-files") or {}).get(group)
    bad = 0
    note = "BELOW the minimum" if stable < floor else f"raise the minimum in differences.yaml to {stable}" if stable > floor else "at the minimum"
    print(f"ratchet {group}: {stable} stable of {total} files ({note}: {floor}); {round(100 * stable / total, 1) if total else 100.0}%")
    bad += stable < floor
    note = "at the recorded value" if recorded == total else f"STALE: set target-files {group} in differences.yaml to {total}"
    print(f"target-files {group}: {total} files ({note}: {recorded})")
    bad += recorded != total
    print("input filters: " + ", ".join(f"{k} {filters[k]}" for k in ("skip",)))
    print(f"{total} files, {total - stable} not stable, {bad} mismatch(es) (stability {group})")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
