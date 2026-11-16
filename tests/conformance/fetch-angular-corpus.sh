#!/usr/bin/env bash
# Fetches the pinned Angular template corpus into tests/conformance/corpus/angular/<repo> (gitignored):
# the .html files of angular/components (MIT), angular/angular (MIT) and the RealWorld app, and
# Prettier's own angular format-test inputs (tag 3.9.10).
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs angular <<'EOF2'
angular/components a4b281d01752685de160df193002c24e40a76b65 *.html
angular/angular 1e6afa4cb84c9194d72d946ba9c22bf740360d9c *.html
gothinkster/angular-realworld-example-app d426ec804f7d035c374aa4a1b48ecdb05a03eb02 *.html
prettier/prettier 2cc4f0fd2cc44a970229a5cb025cd47c63f31b04 */tests/format/angular/*
EOF2
find "$(dirname "$0")/corpus/angular/prettier" -type f ! -name '*.html' -delete
