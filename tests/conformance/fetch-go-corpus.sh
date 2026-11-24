#!/usr/bin/env bash
# Fetches the pinned Go corpus into tests/conformance/corpus/go/<repo> (gitignored): the `.go` files
# of Kubernetes (the source folders, not `vendor`), Hugo, Prometheus and the Go 1.27.4 standard
# library (`go/src`). `vendor` and `testdata` folders are removed: the first is other projects' code,
# the second holds inputs of the projects' tests, many of them no valid Go.
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs go <<'EOT'
kubernetes/kubernetes eb180eecb22eff4ba46879744d439853c657f74f */pkg/*.go */staging/*.go */test/*.go */cmd/*.go */plugin/*.go */third_party/*.go */hack/*.go */cluster/*.go */build/*.go
gohugoio/hugo 7f0b4416ba85844078be57903eec62f61b5ddeb5 *.go
prometheus/prometheus ad8f26eed18a6aabfd9729bb281c6a3e53beabe9 *.go
golang/go 7b2881024c95a11ddb86663a616340aa3a88b67f */src/*.go
EOT
corpus="$(dirname "$0")/corpus/go"
find "$corpus" -type d \( -name vendor -o -name testdata \) -prune -exec rm -rf {} +
