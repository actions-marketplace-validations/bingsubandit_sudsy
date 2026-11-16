#!/usr/bin/env bash
# Fetches the pinned C and C++ corpus into tests/conformance/corpus/cpp/<repo> (gitignored): the
# Linux kernel 6.12 (`kernel/`, `lib/`), SQLite 3.50.4 (`src/`, `ext/`, not the amalgamation), LLVM
# 20.1.8 (support, IR, ADT, clang's Format and Lex), folly 2025.07.07, fmt 11.2.0 and range-v3
# 0.12.0 (C++20), each with its `.clang-format` files.
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs cpp <<'EOT'
torvalds/linux adc218676eef25575469234709c2d87185ca223a */.clang-format */kernel/*.c */kernel/*.h */lib/*.c */lib/*.h
sqlite/sqlite 8ed5e7365e6f12f427910188bbf6b254daad2ef6 */src/*.c */src/*.h */ext/*.c */ext/*.h
llvm/llvm-project 87f0227cb60147a26a1eeb4fb06e3b505e9c7261 */.clang-format */llvm/lib/Support/* */llvm/lib/IR/* */llvm/include/llvm/ADT/* */clang/lib/Format/* */clang/lib/Lex/* */clang/include/clang/Format/*
facebook/folly caef31b93fb165fbe2f3dbe6a4819ef6a1a207c3 */folly/*.cpp */folly/*.h
fmtlib/fmt 40626af88bd7df9a5fb80be7b25ac85b122d6c21 */.clang-format */include/* */src/* */test/*
ericniebler/range-v3 a81477931a8aa2ad025c6bda0609f38e09e4d7ec */.clang-format */include/* */test/* */example/* */perf/*
EOT
