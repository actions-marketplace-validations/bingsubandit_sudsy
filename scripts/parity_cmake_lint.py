"""Runs cmake-lint (cmakelang) on each file of the command line in its own call and prints its report.

cmake-lint stops at the first file it cannot parse and loses the rest of the list; here a crash on one file
is one finding `E9999` (the file is left out of both sides). The package is on PYTHONPATH (parity_tools.json: `install`).
"""
import sys

from cmakelang.lint import __main__ as lint

for path in sys.argv[1:]:
    sys.argv = ["cmake-lint", "--suppress-decorations", path]
    try:
        lint.inner_main()
    except BaseException as error:  # the tool cannot parse the file: a pseudo finding, `unreadable` in parity_tools.json
        print(f"{path}:1: [E9999] cmake-lint failed: {type(error).__name__}")
    sys.stdout.flush()
