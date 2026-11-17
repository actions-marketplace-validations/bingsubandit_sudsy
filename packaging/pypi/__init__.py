"""Runs the `sudsy` binary shipped in this wheel (the C# worker sits beside it)."""
import os
import subprocess
import sys
from pathlib import Path


def main() -> None:
    exe = Path(__file__).parent / "bin" / ("sudsy.exe" if os.name == "nt" else "sudsy")
    if os.name == "nt":
        sys.exit(subprocess.call([str(exe), *sys.argv[1:]]))
    os.execv(exe, [str(exe), *sys.argv[1:]])
