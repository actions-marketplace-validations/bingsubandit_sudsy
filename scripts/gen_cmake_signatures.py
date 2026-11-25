#!/usr/bin/env python3
"""Writes crates/lang-cmake/src/signatures.generated.rs: one `Signature` row per CMake command, read
from the synopses in CMake's own documentation (BSD-3-Clause): `Help/command/*.rst` and the
`.. command::` entries of `Modules/*.cmake`. Keyword lists and structure only, no prose.
usage: python3 -I scripts/gen_cmake_signatures.py <CMake source dir> [<out file>]
The source dir is a checkout or archive of Kitware/CMake (the pin is in the output's header)."""
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
out = Path(sys.argv[2]) if len(sys.argv) > 2 else Path('crates/lang-cmake/src/signatures.generated.rs')
version = {}
for line in (root / 'Source/CMakeVersion.cmake').read_text().splitlines():
    m = re.match(r'set\((CMake_VERSION_\w+) (\d+)\)', line)
    if m:
        version[m.group(1)] = m.group(2)
VERSION = f"{version['CMake_VERSION_MAJOR']}.{version['CMake_VERSION_MINOR']}.{version['CMake_VERSION_PATCH']}"

KEYWORD = re.compile(r'^[A-Z][A-Z0-9_]+$')
TOKEN = re.compile(r'<[^>]*>|\.\.\.|[\[\]|{}()]|"[^"]*"|`[^`]*`_?|[^\s\[\]|{}()<>]+')


def blocks(text):
    """The literal blocks of an rst text: code-block cmake and parsed-literal."""
    lines = text.split('\n')
    res = []
    i = 0
    while i < len(lines):
        sig = re.match(r'^(\s*)\.\. signature::\s*(.*)$', lines[i])
        if sig:
            base = len(sig.group(1))
            body = [sig.group(2)]
            j = i + 1
            while j < len(lines) and lines[j].strip() and not lines[j].strip().startswith(':') and len(lines[j]) - len(lines[j].lstrip()) > base:
                body.append(lines[j])
                j += 1
            res.append('\n'.join(body))
            i = j
            continue
        m = re.match(r'^(\s*)\.\. (code-block:: cmake|parsed-literal::)\s*$', lines[i])
        if m:
            base = len(m.group(1))
            j = i + 1
            body = []
            while j < len(lines) and (not lines[j].strip() or len(lines[j]) - len(lines[j].lstrip()) > base):
                body.append(lines[j])
                j += 1
            res.append('\n'.join(body))
            i = j
        else:
            i += 1
    return res


def substitutions(text):
    subs = {}
    for m in re.finditer(r'^\.\. \|(\w+)\| replace:: (.*)$', text, re.M):
        subs[m.group(1)] = m.group(2).strip()
    return subs


def calls(block, name):
    """The argument text of each call of `name` in a block (parentheses balanced outside <...>)."""
    res = []
    for m in re.finditer(r'(?<![\w`])' + re.escape(name) + r'\s*\(', block, re.I):
        i = m.end()
        depth = 1
        j = i
        angle = 0
        while j < len(block) and depth:
            c = block[j]
            if c == '<':
                angle += 1
            elif c == '>':
                angle = max(0, angle - 1)
            elif not angle and c == '(':
                depth += 1
            elif not angle and c == ')':
                depth -= 1
            j += 1
        res.append(block[i:j - 1])
    return res


class Group(list):
    brace = False


def parse(text):
    stack = [Group()]
    for t in TOKEN.findall(text):
        if t in ('[', '{', '('):
            g = Group()
            g.brace = t == '{'
            stack.append(g)
        elif t in (']', '}', ')'):
            if len(stack) > 1:
                g = stack.pop()
                stack[-1].append(g)
        else:
            stack[-1].append(t)
    while len(stack) > 1:
        g = stack.pop()
        stack[-1].append(g)
    return stack[0]


def is_kw(t):
    return isinstance(t, str) and bool(KEYWORD.match(t)) and not t.startswith('<')


def alt_keywords(t):
    """The keywords of a `<A|B|C>` placeholder, or []."""
    if isinstance(t, str) and t.startswith('<') and '|' in t:
        parts = t[1:-1].split('|')
        if all(KEYWORD.match(p.strip()) for p in parts):
            return [p.strip() for p in parts]
    return []


def has_dots(x):
    if isinstance(x, str):
        return x == '...' or x.endswith('...')
    return any(has_dots(c) for c in x)


def flat(seq):
    for x in seq:
        if isinstance(x, list):
            yield from flat(x)
        else:
            yield x


def kind_after(seq, k):
    """The value kind of the keyword at seq[k]."""
    rest = seq[k + 1:]
    if not rest:
        return 'None'
    n = rest[0]
    if isinstance(n, str) and n == '...':
        return 'Many'
    if isinstance(n, str) and n == '|':
        return 'None'
    if isinstance(n, str) and (n.startswith('<') or not is_kw(n) and n != '|'):
        if n.startswith('<'):
            name = n[1:-1].lower()
            if name.startswith(('cmd', 'command')):
                return 'CommandLine'
            if (re.match(r'.*1$', name) and len(rest) > 1 and isinstance(rest[1], str) and re.match(r'<.*1>$', rest[1])
                    and any(isinstance(r, list) and has_dots(r) or r == '...' for r in rest[2:4])):
                return 'Pairs'
        if n.startswith('command'):
            return 'CommandLine'
        if len(rest) > 1 and (rest[1] == '...' or (isinstance(rest[1], list) and has_dots(rest[1]) and not any(is_kw(x) for x in flat(rest[1])))) or (isinstance(n, str) and n.endswith('...')):
            return 'Many'
        return 'One'
    if isinstance(n, Group) and n.brace:
        return 'Many' if has_dots(n) or (len(rest) > 1 and rest[1] == '...') else 'One'
    if isinstance(n, list):
        inner = list(flat(n))
        if inner and is_kw(inner[0]):
            return 'None'
        if has_dots(n):
            return 'Many'
        if inner and isinstance(inner[0], str) and inner[0].startswith('<'):
            return 'One'
        return 'None'
    return 'None'


RANK = {'None': 0, 'One': 1, 'Many': 2, 'Pairs': 3, 'CommandLine': 3, 'Infix': 4}


def collect(seq, kws, inner_of=None):
    """Adds the keywords of a sequence (and its groups) to kws: word -> [kind, inner kws]."""
    k = 0
    while k < len(seq):
        t = seq[k]
        alts = alt_keywords(t)
        if isinstance(t, Group) and t.brace:
            names = [x for x in t if x != '|']
            if len(names) > 1 and all(is_kw(x) for x in names):
                kind = kind_after([names[0]] + list(seq[k + 1:]), 0)
                for a in names:
                    add(kws, a, kind, None)
            else:
                alt = []
                for x in list(t) + ['|']:
                    if x == '|':
                        if alt and is_kw(alt[0]) and kind_after(alt, 0) in ('Many', 'Pairs'):
                            add(kws, alt[0], kind_after(alt, 0), None)
                            collect(alt[1:], kws)
                        alt = []
                    else:
                        alt.append(x)
        elif isinstance(t, list):
            sub = list(t)
            # a group that starts with an alternation of keywords and holds more keyword groups is a section
            head = [x for x in sub[:1] if isinstance(x, str) and '|' in x]
            if sub and sub[0] is not None and is_alt_group(sub):
                headers = alt_group(sub)
                inner = {}
                collect(sub[len(headers) * 2 - 1:], inner)
                for h in headers:
                    add(kws, h, 'None', inner)
            else:
                collect(sub, kws)
        elif alts:
            for a in alts:
                add(kws, a, 'Many', None)
        elif is_kw(t):
            # an alternation `A|B|C` of keywords
            j = k
            names = [t]
            while j + 2 < len(seq) and seq[j + 1] == '|' and is_kw(seq[j + 2]):
                names.append(seq[j + 2])
                j += 2
            if len(names) > 1:
                for a in names:
                    add(kws, a, 'None', None)
                k = j + 1
                continue
            infix = (0 < k < len(seq) - 1 and isinstance(seq[k - 1], str) and isinstance(seq[k + 1], str)
                     and seq[k - 1].startswith('<cond') and seq[k + 1].startswith('<cond'))
            add(kws, t, 'Infix' if infix else kind_after(seq, k), None)
        k += 1


def is_alt_group(sub):
    return len(sub) >= 3 and is_kw(sub[0]) and sub[1] == '|' and any(isinstance(x, list) for x in sub)


def alt_group(sub):
    names = [sub[0]]
    i = 1
    while i + 1 < len(sub) and sub[i] == '|' and is_kw(sub[i + 1]):
        names.append(sub[i + 1])
        i += 2
    return names if len(names) * 2 - 1 == i else names


def add(kws, word, kind, inner):
    cur = kws.get(word)
    if cur is None or RANK[kind] > RANK[cur[0]]:
        kws[word] = [kind, inner if inner else (cur[1] if cur else None)]
    elif inner and not cur[1]:
        cur[1] = inner


def front(seq):
    """Leading plain parameters before the first keyword or group."""
    n = []
    for k, t in enumerate(seq):
        if isinstance(t, list) or t in ('|', '...') or is_kw(t) or alt_keywords(t):
            break
        if k + 1 < len(seq) and seq[k + 1] == '...':
            break
        if isinstance(t, str) and t.endswith('...'):
            break
        n.append(t)
    return n


def rust_kws(kws, ind):
    if not kws:
        return '&[]'
    pad = ' ' * ind
    items = []
    for w, (kind, inner) in sorted(kws.items()):
        items.append(f'{pad}    Keyword {{ word: "{w}", values: Values::{kind}, inner: {rust_kws(inner, ind + 4)} }},')
    return '&[\n' + '\n'.join(items) + f'\n{pad}]'


def merge(into, other):
    for w, (kind, inner) in other.items():
        add(into, w, kind, inner)


def artifact_groups(text):
    """The kinds of a `<x-kind> <x-option>...` group and the keywords of its options, from the prose
    that lists them: ``Each ``<x-kind> ...`` group applies to`` followed by an indented list of
    ``KIND`` terms, and ``where ``<x-option>...`` group may contain:`` followed by a code block."""
    kinds = []
    m = re.search(r'Each ``<[\w-]+-kind> <[\w-]+-option>\.\.\.`` group applies to.*?\n\n', text, re.S)
    if m:
        for line in text[m.end():].split('\n'):
            t = re.match(r'^  ``([A-Z][A-Z0-9_]+)``\s*$', line)
            if t:
                kinds.append(t.group(1))
            elif line.strip() and not line.startswith(' '):
                break
    options = {}
    o = re.search(r'where ``<[\w-]+-option>\.\.\.`` group may contain:\n\n\s*\.\. code-block:: cmake\n\n(.*?)\n\n(?=\S| {0,2}\S)', text, re.S)
    if o:
        collect(parse(o.group(1)), options)
    return kinds, options


commands = {}   # lowercase name -> (spelling, [blocks text])
artifacts = {}  # lowercase name -> (kinds, option keywords)
cdir = root / 'Help/command'
for f in sorted(cdir.glob('*.rst')):
    text = f.read_text(errors='replace')
    subs = substitutions(text)
    bl = blocks(text)
    for m in re.finditer(r'^\.\. include:: (include/\w+\.rst)', text, re.M):
        inc = cdir / m.group(1)
        if inc.exists():
            bl += blocks(inc.read_text(errors='replace'))
    name = f.stem
    outb = []
    for b in bl:
        b = b.replace('|FIND_XXX|', name)
        for k, v in subs.items():
            b = b.replace(f'|{k}|', v)
        outb.append(b)
    commands[name.lower()] = (name, outb)
    artifacts[name.lower()] = artifact_groups(text)
for f in sorted((root / 'Modules').rglob('*.cmake')):
    text = f.read_text(errors='replace')
    for m in re.finditer(r'^( *)\.\. command:: ([A-Za-z_][A-Za-z0-9_]*)\s*$', text, re.M):
        base = len(m.group(1))
        lines = text[m.end():].split('\n')
        body = []
        for ln in lines[1:]:
            if ln.strip() and len(ln) - len(ln.lstrip()) <= base:
                break
            body.append(ln)
        name = m.group(2)
        commands.setdefault(name.lower(), (name, []))[1].extend(blocks('\n'.join(body)))

rows = []
# `if`, `elseif` and `while` take conditions: only the operators between operands count.
BARE = {'if', 'elseif', 'while'}
cond = {}
for b in commands['if'][1]:
    for c in calls(b, 'if'):
        collect(parse(c), cond)
cond = {w: k for w, k in cond.items() if k[0] == 'Infix'}
for low in sorted(commands):
    name, bl = commands[low]
    if low in BARE:
        rows.append(f'    Signature {{\n        name: "{name}",\n        keywords: {rust_kws(cond, 8)},\n        ..Signature::NONE\n    }},')
        continue
    base_kws = {}
    forms = {}
    front_n = None
    seconds = {}
    for b in bl:
        for c in calls(b, name):
            sq = parse(c)
            if len(sq) > 1 and is_kw(sq[0]) and is_kw(sq[1]) and not (len(sq) > 2 and sq[2] == '|'):
                seconds.setdefault(sq[0], set()).add(sq[1])
    kind_forms = set()
    kind_text = ''
    for b in bl:
        for c in calls(b, name):
            # an example (no placeholder, no optional part) is not a synopsis
            if '<' not in c and '[' not in c:
                continue
            seq = parse(c)
            if not seq:
                continue
            if is_kw(seq[0]) and len(bl) > 1:
                words = [seq[0]]
                if len(seq) > 1 and is_kw(seq[1]) and len(seconds.get(seq[0], ())) > 1 and not (len(seq) > 2 and seq[2] == '|'):
                    words.append(seq[1])
                rest = seq[len(words):]
                kws = {}
                collect(rest, kws)
                f = forms.setdefault(tuple(words), [None, {}])
                if '-kind>' in b:
                    kind_forms.add(tuple(words))
                    kind_text += b
                f[0] = len(front(rest)) if f[0] is None else min(f[0], len(front(rest)))
                merge(f[1], kws)
            else:
                if front_n is None:
                    front_n = [re.sub(r'[^A-Za-z0-9_-]', '', x) or 'arg' for x in front(seq)]
                collect(seq, base_kws)
    kinds, options = artifacts.get(low, ([], {}))
    for words in kind_forms:
        kws = forms[words][1]
        for w, (kind, inner) in options.items():
            add(kws, w, kind, inner)
        opts = '|'.join(options) or '-'
        followed = [w for w in kws if kws[w][0] == 'None' and re.search(r'\b%s\s+(%s)\b' % (w, opts), kind_text)]
        for w in kinds + followed:
            kws[w] = ['None', {x: list(v) for x, v in options.items()}]
    pos = ', '.join(f'"{x}"' for x in (front_n or []))
    fr = []
    for words, (n, kws) in sorted(forms.items()):
        w = ', '.join(f'"{x}"' for x in words)
        fr.append(f'            Form {{ words: &[{w}], positional: {n or 0}, keywords: {rust_kws(kws, 12)} }},')
    forms_s = '&[\n' + '\n'.join(fr) + '\n        ]' if fr else '&[]'
    rows.append(f'    Signature {{\n        name: "{name}",\n        positional: &[{pos}],\n        keywords: {rust_kws(base_kws, 8)},\n        forms: {forms_s},\n        ..Signature::NONE\n    }},')

header = (f"// Generated by scripts/gen_cmake_signatures.py from the documentation of CMake {VERSION} (Kitware/CMake 4207edd6abcaa33719037ec1095d7767f4c684b4:\n"
          "// Help/command/*.rst and the `.. command::` entries of Modules/*.cmake): the keywords of each command's synopsis, nothing else.\n"
          "use core_syntax::signature::{Form, Keyword, Signature, Values};\n\n"
          "pub static SIGNATURES: &[Signature] = &[\n")
out.write_text(header + '\n'.join(rows) + '\n];\n', newline='\n') if False else open(out, 'w', newline='\n').write(header + '\n'.join(rows) + '\n];\n')
print(len(rows), 'commands', file=sys.stderr)
