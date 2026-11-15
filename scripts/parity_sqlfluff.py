"""Runs the sqlfluff installed by parity_format.py (`pip install --target <prefix>`): parity_sqlfluff.py <prefix> <file>.

`sqlfluff fix` with the layout and capitalisation rules, in the dialect that starts the file name
(`<dialect>__<name>.sql`). A file with a parse error (rule PRS) exits 2, so the runner counts it as rejected."""
import io
import sys
from contextlib import redirect_stderr, redirect_stdout

sys.path.insert(0, sys.argv[1])
from sqlfluff.cli.commands import cli

name = sys.argv[2]
out = io.StringIO()
with redirect_stdout(out), redirect_stderr(out):
    status = cli(["fix", "--rules", "layout,capitalisation", "--dialect", name.split("__")[0], "--nocolor", name], standalone_mode=False)
sys.exit(2 if " PRS " in out.getvalue() else 0 if status in (0, 1, None) else status)
