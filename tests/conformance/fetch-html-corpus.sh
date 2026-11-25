#!/usr/bin/env bash
# Fetches the pinned HTML corpus into tests/conformance/corpus/html/<repo> (gitignored): Prettier's own
# html and vue format-test inputs (tag 3.9.10; the .html and .vue files, no snapshots or runners) and real
# pages: Bootstrap 5.3.3 (its JS test pages and docs site, whose Hugo templates are not HTML) and
# mdn/learning-area (a 128 MB tarball), the hand-written demo pages of mdn's dom, webaudio, web-components, pwa, js, html
# and webgl examples (CC0), Web-Dev-For-Beginners (MIT) and html5-boilerplate (MIT); Vue (Vue core, VitePress, Pinia, Nuxt UI) and Svelte (the realworld app,
# SvelteKit, the Prettier plugin's samples) sources.
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs html <<'EOF'
prettier/prettier 1172b82c295c4f889fd787dc6c78c3e194ebf353 */tests/format/html/* */tests/format/vue/*
twbs/bootstrap 6e1f75f420f68e1d52733b8e407fc7c3766c9dba */js/tests/*.html */site/*.html
mdn/learning-area 21973263ba859ff3477919087b488157f9d618a6 *.html
vuejs/core 7ef6817112bb420ef47a7252821b37caae016ac9 *.vue
sveltejs/prettier-plugin-svelte 44dda204c660fafbf0e5c6c71b6f6c3c4423adce */test/formatting/samples/*/input.html
sveltejs/realworld df796708040f5200ec572b28ab7f88ecee5794dd *.svelte
vuejs/vitepress 40b2e090772ac11eac358d25ec92ee435fc2adbd *.vue
vuejs/pinia 91f918e59dddae4cc2609ab3d8e5607e29fd442f *.vue
nuxt/ui 550471d1a2c1178c9cc9593d8ff684cae7d64674 *.vue
sveltejs/kit a58e0a92c35dbeee161e96c733464dc7397e073a *.svelte
mdn/dom-examples a2c57197c219bd1c87d48aa901f4c2a24b249658 *.html
mdn/webaudio-examples 5582dbc39f241117c27c516b21243b041dc69fcb *.html
mdn/web-components-examples 7e588e43bb8ed439716c413a3bd42466112155c8 *.html
mdn/pwa-examples 8bb6c6f5e4672f0295f1a8ba749a3b6c1f780708 *.html
mdn/js-examples 0dc0081d4beb71346b2e27c62e3b2d29d3e20d35 *.html
mdn/html-examples 57ad3ae9d04c0fab8e3fbf9bcb90d8e3af348e20 *.html
mdn/webgl-examples 17a7484b5bb5cddee1dce3eb93c8d48925453557 *.html
microsoft/Web-Dev-For-Beginners 93b9967af14efd43246c713217dad40d7c27c2f5 *.html
h5bp/html5-boilerplate 7fa0bd6c9e6818d11a1a7efad56fe715ae56d848 *.html
EOF
# Built output of the example projects is not a hand-written source.
find "$(dirname "$0")/corpus/html" -type d \( -name node_modules -o -name dist \) -prune -exec rm -rf {} +
prettier="$(dirname "$0")/corpus/html/prettier"
find "$prettier" -name __snapshots__ -prune -exec rm -rf {} +
find "$prettier" -type f ! -regex '.*\.\(html\|htm\|vue\)' -delete
# The plugin's samples are named input.html and read as Svelte.
find "$(dirname "$0")/corpus/html/prettier-plugin-svelte" -name input.html | while read -r f; do mv "$f" "${f%.html}.svelte"; done
