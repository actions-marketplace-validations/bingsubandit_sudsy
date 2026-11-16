#!/usr/bin/env bash
# Fetches the pinned Kotlin corpus into tests/conformance/corpus/kotlin/<repo> (gitignored):
# kotlinx.coroutines, ktor, nowinandroid, ktlint itself and the Kotlin repository's standard
# library (`.kt` and Gradle `.kts` files), plus the Kotlin compiler's PSI parser test inputs
# (`compiler/psi/psi-impl/testData/psi`, with the expected trees of the compiler as `*.txt`; they
# hold deliberate syntax errors, so the conformance test skips them and the parity harness uses
# them).
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs kotlin <<'EOT'
Kotlin/kotlinx.coroutines 2ac8def478229b9cacd2c18f07a2e83a10a3aa84 *.kt *.kts
ktorio/ktor cc3d345bc2013d91f394d4d7d7062d647374d711 *.kt *.kts
android/nowinandroid 2982e63cb691ad49cd009aab43dc206fee6699c2 *.kt *.kts
ktlint/ktlint 86855bc8571d3582c62f99f04a8abe57e21221e6 *.kt *.kts
JetBrains/kotlin 71dda614f704fab5a5c21b58c61b1b701a49baee */libraries/stdlib/*.kt */libraries/stdlib/*.kts */compiler/psi/psi-impl/testData/psi/*
EOT
