"""Facts of the Gradle format evidence (docs/ai/design/gradle.md) for scripts/evidence.py: corpus, the rows of the decision table.

`python3 -I scripts/evidence.py gradle table` prints the decision table with the vote of the deciding projects and the
held-out agreement of the form each row names in SUDSY."""
import re

SEED = 20261204
NAMES = re.compile(r".*\.gradle$")
DIRS = [("gradle", NAMES)]
PROJECT = {}
CAP = 60
KEEP_EOL = True
TOOL = ["format", "--stdin"]
KLASSES = {}

# row -> the variant (or variants) a formatter writes; None: kept as written (below the 60% of the rule of no-reference-tool.md)
SUDSY = {
    "indent of a block body": "4 spaces",
    "indent character": "space",
    "quotes of a plain string": None,
    "dependency declaration": None,
    "blank before `{` of a block": "one",
    "blanks around `=` of an assignment": "both",
    "blank after `:` of a named argument": "after only",
    "blank after a comma": "one",
    "blank after `if`/`for`/`while`/`switch`": "one",
    "`else` placement": "after `}`",
    "blanks inside a one-line closure": "inside",
    "continuation line indent (after a trailing comma)": None,
    "blank after `//`": "one",
    "blank lines in a row": {"one", "two"},
    "blank line after `{`": "none",
    "trailing blanks of a file": "none",
    "final newline": "yes",
    "line endings": "LF",
    "trailing semicolon": "none",
    "line width": None,
}


def mask(text):
    """(code, strings, comments): the text with comment bodies and string contents replaced by blanks / `x` (same length, newlines kept)."""
    out, strings, comments, i, n = [], [], [], 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i) or (c == "#" and i == 0 and text.startswith("#!")):
            j = text.find("\n", i)
            j = n if j < 0 else j
            comments.append(text[i:j])
            out.append(" " * (j - i))
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j
        elif text.startswith(("'''", '"""'), i):
            q = text[i:i + 3]
            j = text.find(q, i + 3)
            j = n if j < 0 else j + 3
            out.append(q + "".join(ch if ch == "\n" else "x" for ch in text[i + 3:j - 3]) + q)
            i = j
        elif c in "'\"":
            j = i + 1
            while j < n and text[j] != c and text[j] != "\n":
                j += 2 if text[j] == "\\" else 1
            body = text[i + 1:j]
            strings.append((c, body))
            out.append(c + "x" * len(body) + (c if j < n and text[j] == c else ""))
            i = j + 1 if j < n and text[j] == c else j
        else:
            out.append(c)
            i += 1
    return "".join(out), strings, comments


DEP = re.compile(r"^\s*(?:api|implementation|compileOnly|runtimeOnly|testImplementation|testCompileOnly|testRuntimeOnly|annotationProcessor|kapt|compile|testCompile|classpath|androidTestImplementation|debugImplementation)([( ])\s*['\"]")
BLOCK = re.compile(r"[\w)\]\"']\s*\{\s*$|[\w)\]\"'](\s*)\{")


def variants(text):
    v = {k: [] for k in SUDSY}
    code, strings, comments = mask(text)
    lines = code.split("\n")
    raw = text.split("\n")
    for q, body in strings:
        if "$" not in body and "\\" not in body and not (q == "'" and '"' in body) and not (q == '"' and "'" in body) and "\n" not in body:
            v["quotes of a plain string"].append("single" if q == "'" else "double")
    v["final newline"].append("yes" if text.endswith("\n") else "no")
    v["line endings"].append("CRLF" if "\r\n" in text else "LF")
    v["trailing blanks of a file"].append("present" if any(re.search(r"[ \t]+\r?$", l) for l in raw) else "none")
    run = 0
    for k, l in enumerate(lines):
        l = l.rstrip("\r")
        if not l.strip():
            run += 1
            continue
        if run:
            v["blank lines in a row"].append({1: "one", 2: "two"}.get(run, "three or more"))
        if k and run == 1 and lines[k - 1].rstrip().endswith("{") and lines[k - 1].strip() != "{":
            v["blank line after `{`"].append("blank")
        elif k and not run and lines[k - 1].rstrip().endswith("{") and lines[k - 1].strip() != "{":
            v["blank line after `{`"].append("none")
        run = 0
        ind = re.match(r"[ \t]*", l).group()
        if ind:
            v["indent character"].append("tab" if "\t" in ind else "space")
        if k and lines[k - 1].rstrip().endswith("{") and lines[k - 1].strip() and not lines[k - 1].strip().startswith("}"):
            pind = re.match(r"[ \t]*", lines[k - 1]).group()
            if ind.startswith(pind) and len(ind) > len(pind):
                d = ind[len(pind):]
                v["indent of a block body"].append("tab" if "\t" in d else f"{len(d)} spaces")
        else:
            if k > 1 and lines[k - 1].rstrip().endswith(",") and not lines[k - 2].rstrip().endswith(",") and lines[k - 1].strip():
                pind = re.match(r"[ \t]*", lines[k - 1]).group()
                d = len(ind) - len(pind)
                v["continuation line indent (after a trailing comma)"].append({0: "same", 2: "+2", 4: "+4", 8: "+8"}.get(d, "other"))
        m = DEP.match(l)
        if m:
            v["dependency declaration"].append("parens" if m.group(1) == "(" else "no parens")
        for m in re.finditer(r"[\w)\]](\s*)\{", l):
            if re.search(r"\$\s*$", l[:m.start() + 1]):
                continue
            v["blank before `{` of a block"].append("none" if not m.group(1) else "one" if m.group(1) == " " else "several")
        m = re.match(r"\s*[\w.\[\]]+(\s*)=(\s*)[^=~]", l)
        if m and not l.lstrip().startswith(("==", "=")):
            a, b = bool(m.group(1)), bool(m.group(2))
            v["blanks around `=` of an assignment"].append("both" if a and b else "none" if not a and not b else "after only" if b else "before only")
        for m in re.finditer(r"(?:^|[(,\[]\s*)(\w+)(\s*):(\s*)(?=[^\s:])", l):
            a, b = bool(m.group(2)), bool(m.group(3))
            v["blank after `:` of a named argument"].append("after only" if b and not a else "none" if not a else "both" if b else "before only")
        for m in re.finditer(r",(\s*)(?=\S)", l):
            v["blank after a comma"].append("one" if m.group(1) == " " else "none" if not m.group(1) else "several")
        for m in re.finditer(r"\b(?:if|for|while|switch)(\s*)\(", l):
            v["blank after `if`/`for`/`while`/`switch`"].append("one" if m.group(1) else "none")
        if re.match(r"\s*else\b", l) and k and lines[k - 1].strip():
            v["`else` placement"].append("own line" if lines[k - 1].strip().endswith("}") else "after `}`")
        elif re.search(r"\}\s*else\b", l):
            v["`else` placement"].append("after `}`")
        for m in re.finditer(r"\{([^{}\n]*)\}", l):
            inner = m.group(1)
            if inner.strip():
                v["blanks inside a one-line closure"].append("inside" if inner.startswith(" ") and inner.endswith(" ") else "none" if not inner.startswith(" ") and not inner.endswith(" ") else "one side")
        if l.rstrip().endswith(";"):
            v["trailing semicolon"].append("semicolon")
        elif l.strip() and not l.rstrip().endswith(("{", "}", ",", "(", "[")):
            v["trailing semicolon"].append("none")
        w = len(raw[k].rstrip("\r").expandtabs(4))
        v["line width"].append("<=80" if w <= 80 else "81-100" if w <= 100 else "101-120" if w <= 120 else ">120")
    for c in comments:
        if c.startswith("//") and len(c) > 2 and not c.startswith(("///", "//-", "//=", "//*", "// ")) and c[2].isalnum():
            v["blank after `//`"].append("none")
        elif c.startswith("// "):
            v["blank after `//`"].append("one")
    return v
