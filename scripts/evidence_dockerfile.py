"""Facts of the Dockerfile format evidence (docs/ai/design/dockerfile.md) for scripts/evidence.py: corpus, the rows of the decision table, the causes of difference to dockerfmt."""
import re
from collections import defaultdict
from difflib import SequenceMatcher

SEED = 20261125
NAMES = re.compile(r"^(Dockerfile(\..*)?|.*\.[Dd]ockerfile)$")
DIRS = [("dockerfile", NAMES), ("dockerfile-evidence", NAMES)]
TOOL = []  # dockerfmt reads the file on stdin
KLASSES = {"shell": "shell code of RUN/CMD/ENTRYPOINT/HEALTHCHECK", "rewrite": "legacy ENV, COPY JSON, MAINTAINER rewrites"}

# Project of a repository directory when it is not the directory name: the docker-library images and Docker's own
# repositories are one project each, so a project is never on both halves.
PROJECT = {"golang": "docker-library", "python": "docker-library", "nginx": "docker-library", "postgres": "docker-library",
           "buildx": "moby", "docker": "nextcloud", "core": "home-assistant", "docker-node": "nodejs", "dotnet-docker": "dotnet"}
SHELL_KW = {"RUN", "CMD", "ENTRYPOINT"}
# Causes that rewrite syntax instead of laying it out (Sudsy changes only case and blanks)
REWRITES = ("ENV: legacy", "COPY: JSON", "ADD: JSON", "instruction replaced")

def parse(text):
    """Items of a Dockerfile: ('blank', n), ('comment', line), ('ins', record). Independent of the formatter."""
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    esc = "\\"
    for l in lines:
        m = re.match(r"\s*#\s*escape\s*=\s*(\S)", l, re.I)
        if m:
            esc = m.group(1)
        elif not re.match(r"\s*#\s*\w+\s*=", l):
            break
    items, i = [], 0
    while i < len(lines):
        l = lines[i]
        if not l.strip():
            n = 0
            while i < len(lines) and not lines[i].strip():
                n, i = n + 1, i + 1
            items.append(("blank", n))
        elif l.lstrip().startswith("#"):
            items.append(("comment", l)),
            i += 1
        else:
            phys = [l]
            i += 1
            cont = l.rstrip().endswith(esc)
            while cont and i < len(lines):
                phys.append(lines[i])
                if lines[i].strip() and not lines[i].lstrip().startswith("#"):
                    cont = lines[i].rstrip().endswith(esc)
                i += 1
            heredoc = []
            logical = " ".join(p.rstrip().rstrip(esc) for p in phys if not p.lstrip().startswith("#"))
            for m in re.finditer(r"(?<!<)<<(?!<)-?\s*([\"']?)(\w+)\1", logical):
                end = next((k for k in range(i, len(lines)) if lines[k].strip() == m.group(2)), None)
                if end is not None:
                    heredoc.append(lines[i:end + 1])
                    i = end + 1
            kw = phys[0].split(None, 1)[0] if phys[0].split() else ""
            items.append(("ins", {"kw": kw, "phys": phys, "esc": esc, "heredoc": heredoc, "logical": logical}))
    return items


def variants(path_text):
    """{row: [variant per occurrence]} of one file."""
    text = path_text
    rows = defaultdict(list)
    rows["final newline"].append("yes" if text.endswith("\n") else "no")
    items = parse(text)
    for kind, v in items:
        if kind == "blank":
            rows["blank lines"].append("one" if v == 1 else "several")
        if kind != "ins":
            continue
        kw, phys, esc = v["kw"], v["phys"], v["esc"]
        rows["keyword case"].append("upper" if kw.isupper() else "lower" if kw.islower() else "mixed")
        if kw.upper() == "FROM":
            m = re.search(r"\s(as)\s", v["logical"], re.I)
            if m:
                rows["AS"].append("upper" if m.group(1) == "AS" else "lower" if m.group(1) == "as" else "mixed")
        first = phys[0].rstrip()
        if first.endswith(esc):
            first = first[:-1]
        body = first[len(phys[0]) - len(phys[0].lstrip()) + len(kw):]
        unq = re.sub(r'"[^"]*"|\'[^\']*\'', "Q", body)
        args = unq.strip()
        if kw.upper() in SHELL_KW:
            gaps = [re.match(r"\s*", unq).group(0)]
        else:
            gaps = [re.match(r"\s*", unq).group(0)] + re.findall(r"(?<=\S)(\s+)(?=\S)", args)
        if gaps and gaps[0]:
            rows["blanks between arguments"].append("single" if all(g == " " for g in gaps) else "several")
        j = args.lstrip()
        if j.startswith("[") and kw.upper() != "ONBUILD":
            if re.search(r"\[\s|\s\]", j.replace("Q", "q")) or re.search(r'\[\s|\s\]', body.strip()):
                rows["JSON array"].append("blanks inside brackets")
            elif re.search(r"\",\S|',\S|Q,\S", body):
                rows["JSON array"].append("no blank after comma")
            elif re.search(r"[\"'],\s{2,}|\t", body):
                rows["JSON array"].append("several blanks")
            else:
                rows["JSON array"].append("one blank after comma")
        cont = [p for p in phys[1:] if not p.lstrip().startswith("#") and p.strip()]
        for n, p in enumerate(phys):
            if p.rstrip().endswith(esc) and not p.lstrip().startswith("#"):
                pre = p.rstrip()[:-1]
                gap = len(pre) - len(pre.rstrip(" \t"))
                rows["blanks before backslash"].append("one" if gap == 1 and pre.rstrip() == pre.rstrip(" \t") and not pre.endswith("\t") else "none" if gap == 0 else "several")
        ind = [re.match(r"[ \t]*", p).group(0) for p in cont]
        for p in cont:
            if re.search(r"(&&|\|\|)\s*\\$", p.rstrip()) :
                rows["`&&`, `||` at line end or start"].append("end")
            elif re.match(r"\s*(&&|\|\|)", p):
                rows["`&&`, `||` at line end or start"].append("start")
        for g in ind:
            rows["continuation indent"].append("tab" if "\t" in g else f"{len(g)} spaces")
        for p in phys[1:]:
            if p.lstrip().startswith("#"):
                g = re.match(r"[ \t]*", p).group(0)
                rows["comment in continuation"].append("column 0" if not g else "indented")
        for hl in v["heredoc"]:
            rows["heredoc"].append("as written")
    return rows


SUDSY = {"keyword case": "upper", "AS": "upper", "blanks between arguments": "single", "JSON array": "one blank after comma",
         "blanks before backslash": "one", "continuation indent": "4 spaces", "comment in continuation": "indented",
         "blank lines": "one", "final newline": "yes", "heredoc": "as written", "`&&`, `||` at line end or start": None}




def norm(s):
    return re.sub(r"\s+", " ", re.sub(r"\\\n", " ", s)).strip()


def compact(s):
    return re.sub(r"\s+", "", s.replace("\\", ""))


def shell_ins(v):
    if v["kw"].upper() == "HEALTHCHECK":
        return bool(re.search(r"\bCMD\s+[^\[\s]", v["logical"]))
    return v["kw"].upper() in SHELL_KW and not re.match(r"\s*\w+\s+\[", v["logical"])


def item_text(it):
    if it[0] == "ins":
        return "\n".join(it[1]["phys"] + [l for h in it[1]["heredoc"] for l in h])
    return it[1] if it[0] == "comment" else ""


def raw_causes(a, b):
    """[(cause, is_shell)] of the differences between two outputs a (Sudsy) and b (dockerfmt); items aligned by content."""
    ia, ib = [x for x in parse(a) if x[0] != "blank"], [x for x in parse(b) if x[0] != "blank"]
    out = []
    ba, bb = [x[1] for x in parse(a) if x[0] == "blank"], [x[1] for x in parse(b) if x[0] == "blank"]
    if ba != bb:
        out.append(("blank lines between instructions", False))
    ka = [compact(item_text(x)).lower() for x in ia]
    kb = [compact(item_text(x)).lower() for x in ib]
    sm = SequenceMatcher(None, ka, kb, autojunk=False)
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal":
            out += [classify(x, y) for x, y in zip(ia[i1:i2], ib[j1:j2]) if item_text(x) != item_text(y)]
            continue
        pairs = list(zip(ia[i1:i2], ib[j1:j2])) if (i2 - i1) == (j2 - j1) else []
        if not pairs:
            for x in ia[i1:i2] + ib[j1:j2]:
                out.append(("instruction added/removed/rewritten (%s)" % (x[1]["kw"].upper() if x[0] == "ins" else x[0]), x[0] == "ins" and shell_ins(x[1])))
            continue
        for x, y in pairs:
            out.append(classify(x, y))
    return out



def causes(a, b):
    """[(cause, class)]: shell code and the rewrites of syntax are classes of their own."""
    return [(c, "shell" if shell else "rewrite" if c.startswith(REWRITES) else "") for c, shell in raw_causes(a, b)]


EXAMPLES = {}


def classify(x, y):
    c = classify1(x, y)
    EXAMPLES.setdefault(c[0], []).append((item_text(x), item_text(y)))
    return c


def classify1(x, y):
    if x[0] == "comment" or y[0] == "comment":
        return ("comment: blank after `#`, indent", False)
    vx, vy = x[1], y[1]
    sh = shell_ins(vx) or shell_ins(vy)
    kw = vy["kw"].upper()
    if vx["kw"].upper() != kw:
        return (f"instruction replaced ({vx['kw'].upper()} -> {kw})", False)
    if sh:
        la, lb = [p.strip() for p in vx["phys"]], [p.strip() for p in vy["phys"]]
        if compact(vx["logical"]) == compact(vy["logical"]):
            if la == lb:
                return (f"{kw}: indent of continued lines", True)
            if any(l.endswith("&& \\") for l in la) and any(l.startswith("&&") for l in lb):
                return (f"{kw}: `&&` and `||` moved from line end to line start", True)
            return (f"{kw}: lines joined or split", True)
        if re.sub(r"[;&|<>\\\s]", "", vx["logical"]) == re.sub(r"[;&|<>\\\s]", "", vy["logical"]):
            return (f"{kw}: shell separators, redirect blanks, case layout", True)
        return (f"{kw}: shell text rewritten (quotes, heredoc, comments)", True)
    if norm(vx["logical"]) == norm(vy["logical"]):
        return (f"{kw}: continuation, indent or blank layout", False)
    if norm(vx["logical"]).lower() == norm(vy["logical"]).lower():
        return (f"{kw}: keyword or stage name case", False)
    if kw == "ENV" and re.match(r"ENV\s+[^\s=]+\s+\S", vx["logical"].strip(), re.I) and "=" not in vx["logical"].split()[1]:
        return ("ENV: legacy `ENV key value` written as `key=value`", False)
    if kw in ("COPY", "ADD") and "[" in vx["logical"] and "[" not in vy["logical"]:
        return (f"{kw}: JSON array written as plain arguments", False)
    if kw in ("ENV", "ARG", "LABEL"):
        return (f"{kw}: quoting or pair layout", False)
    return (f"{kw}: argument text rewritten", False)


