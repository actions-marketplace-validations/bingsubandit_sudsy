#!/usr/bin/env bash
# Fetches 20 more pinned projects for the Makefile format evidence (docs/ai/design/makefile.md) into
# tests/conformance/corpus/makefile-evidence/<repo> (gitignored); the projects were chosen before any result was seen.
# Every `Makefile`, `GNUmakefile`, `makefile`, `*.mk` and `*.mak` of a project, except vendored and test inputs; the
# kernel-style trees (u-boot, buildroot, openwrt) are cut to their build system, not their thousands of per-directory
# `obj-y +=` files. Together with fetch-makefile-corpus.sh they make the 26 projects of the evidence.
export TARBALL_LENIENT=1
source "$(dirname "$0")/tarball-corpus.sh"
P='*/Makefile */GNUmakefile */makefile */*.mk */*.mak !*/vendor/* !*/node_modules/* !*/third_party/* !*/testdata/* !*/test/fixtures/*'
fetch_tarballs makefile-evidence <<EOF
FFmpeg/FFmpeg 5573a5bcc8437edc20cfae3277dd6bdde002e7a9 $P
vim/vim 0676bbf7bdec887ab09d7fb627d42fced3c7fc8e $P
micropython/micropython b9620707e3697def3d8829a365aa0c863a69974e $P
postgres/postgres 4255812efdbcf7f1acf6be2211e1b5b11c4d4240 $P !*/contrib/* !*/src/test/*
openwrt/openwrt 9285edcb617278c1aa982d917a34460c05f6e5f3 $P !*/package/* !*/target/*
ocaml/ocaml bc1c440270a4bdaeae1965cf57ac9174a67adba3 $P
nodejs/node c08b6c39c5e90df5d26a8441ca528e6a61b5b94b $P !*/deps/*
neovim/neovim 722cb01277826227406afc0fe3548629251de264 $P
cockroachdb/cockroach d30c905fff79ef825adc96bcc647f1872a90f2ff $P !*/c-deps/*
etcd-io/etcd 4e090c033abc29636df2f844fbdd363adffcd90b $P
helm/helm 8edbd3cc1894e69fe0e7caa2f2e68fbf2ee476a8 $P
containerd/containerd c3e6f5eed3693f4fc3cb3656c34a09c47b3e9ad2 $P
hashicorp/terraform 279a0c2487913cb700da99fd5077d8ef7d486f04 $P
grafana/grafana 55b2baafe8c9a399c44199d32fcb20f280e352bc $P
golangci/golangci-lint cfaa38e432762a1f3a067c7a5cb92237fce614bf $P
numpy/numpy 2b8e730aafe72c25e71ad07db10b4fb494ed9682 $P
sphinx-doc/sphinx 035a07c32c3241fd3356f4d827800c1f88525272 $P !*/tests/*
u-boot/u-boot 9d67454e920b3908529c9fc642a8c911db040896 $P !*/arch/* !*/board/* !*/drivers/* !*/configs/* !*/lib/* !*/fs/* !*/net/* !*/common/* !*/cmd/* !*/doc/* !*/test/* !*/dts/* !*/include/* !*/boot/* !*/disk/* !*/env/* !*/post/* !*/api/* !*/examples/*
buildroot/buildroot a5f3c5d1a42bec0df855753e6c85848d8fdf5746 $P !*/package/* !*/board/* !*/configs/* !*/boot/* !*/fs/* !*/linux/* !*/toolchain/* !*/arch/*
python/cpython 3b3093726ffd07ebba5b906a1a55e453532e78b7 $P
EOF
