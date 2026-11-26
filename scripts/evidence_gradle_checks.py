"""Yield of candidate lint-checklist entries over the capped Gradle corpus (docs/ai/design/gradle.md): occurrences, files and
projects per pattern. The patterns are the syntactic ones of the Gradle upgrade guides and well known pitfalls; each is
matched on the code with comments and string contents masked (evidence_gradle.mask) unless it needs the string.
Usage: python3 -I scripts/evidence_gradle_checks.py"""
import re
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence
import evidence_gradle as mod

# id: (regex on the unmasked text of a line with comments removed, source)
CHECKS = {
    "removed configuration compile/testCompile/runtime (Gradle 7)": (r"^\s*(?:compile|testCompile|runtime|testRuntime|debugCompile|releaseCompile)\s*[(\"' ]", "Gradle 7 upgrade guide"),
    "jcenter() (sunset by JFrog)": (r"\bjcenter\s*\(", "Gradle 6.x deprecation, JFrog sunset"),
    "insecure http:// repository url": (r"\burl\s*[=(]?\s*['\"]http://", "Gradle 7 upgrade guide (insecure protocol)"),
    "dynamic version (+ or latest.*)": (r"['\"][\w.\-]+:[\w.\-]+:[\w.\-]*\+['\"]|latest\.(?:release|integration)", "Gradle dependency docs, reproducible builds"),
    "task x << { } (left shift, removed in Gradle 5)": (r"^\s*task\s+\w+\s*<<", "Gradle 5 upgrade guide"),
    "archive task baseName/classifier/extension/version (removed in Gradle 8)": (r"^\s*(?:baseName|classifier|extension|appendix)\s*[= ]", "Gradle 8 upgrade guide"),
    "mainClassName (removed in Gradle 8)": (r"\bmainClassName\s*=", "Gradle 7 upgrade guide"),
    "uploadArchives / maven plugin (removed in Gradle 7)": (r"\buploadArchives\b|apply\s+plugin\s*:\s*['\"]maven['\"]", "Gradle 7 upgrade guide"),
    "buildDir (deprecated in Gradle 8.3)": (r"\bbuildDir\b", "Gradle 8.3 deprecation"),
    "tasks.create / task x(type:) instead of register (task configuration avoidance)": (r"\btasks\.create\s*\(", "Gradle task configuration avoidance guide"),
    "apply plugin: (legacy; plugins block)": (r"^\s*apply\s+plugin\s*:", "Gradle plugin docs"),
    "Android compileSdkVersion/minSdkVersion/targetSdkVersion (renamed)": (r"^\s*(?:compileSdkVersion|minSdkVersion|targetSdkVersion)\b", "Android Gradle Plugin API"),
    "Android lintOptions (renamed to lint)": (r"^\s*lintOptions\s*\{", "Android Gradle Plugin API"),
    "Android buildToolsVersion (unneeded)": (r"^\s*buildToolsVersion\b", "Android Gradle Plugin API"),
    "convention sourceCompatibility/targetCompatibility outside java {}": (r"^\s*(?:sourceCompatibility|targetCompatibility)\b", "Gradle 8.x deprecation of the Java convention"),
}

corpus = evidence.files(mod)
deciding, held = evidence.split(corpus, mod.SEED)
res = defaultdict(lambda: [0, set(), set(), 0, set()])
for proj, name, f in corpus:
    text = evidence.read(f, True)
    if text is None:
        continue
    code = re.sub(r"//[^\n]*", "", re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group().count("\n"), text, flags=re.S))
    for cid, (rx, _) in CHECKS.items():
        n = len(re.findall(rx, code, flags=re.M))
        if n:
            r = res[cid]
            r[0] += n
            r[1].add(name)
            r[2].add(proj)
            if proj in held:
                r[3] += n
                r[4].add(proj)
print(f"{len(corpus)} files, {len({p for p, _, _ in corpus})} projects ({len(held)} held out)\n")
print("| entry | source | occurrences | files | projects | held-out occurrences |\n|---|---|---|---|---|---|")
for cid, (rx, src) in CHECKS.items():
    r = res.get(cid, [0, set(), set(), 0, set()])
    print(f"| {cid} | {src} | {r[0]} | {len(r[1])} | {len(r[2])} | {r[3]} |")
