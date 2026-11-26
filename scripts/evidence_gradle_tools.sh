#!/usr/bin/env bash
# Installs the tools of the Gradle evidence (docs/ai/design/gradle.md) under WORK (default: <slot>/gradle-eval): a Temurin JDK,
# npm-groovy-lint, a Gradle distribution with a Spotless project (greclipse over a copy of the corpus), the tree-sitter grammar
# candidates. Safe to rerun (a finished part is skipped). Versions are printed; the doc records them.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SLOT="${CARGO_TARGET_DIR:-$(cat "$ROOT/.cargo/tmp/SLOT")}"
WORK="${1:-$SLOT/gradle-eval}"
mkdir -p "$WORK"; cd "$WORK"

if [ ! -x jdk/bin/java ]; then
  rm -rf jdk jdk.tgz; mkdir jdk
  curl -sSfL -o jdk.tgz "https://api.adoptium.net/v3/binary/latest/21/ga/linux/$(uname -m | sed s/x86_64/x64/)/jdk/hotspot/normal/eclipse"
  tar -xzf jdk.tgz -C jdk --strip-components=1; rm jdk.tgz
fi
export JAVA_HOME="$WORK/jdk" PATH="$WORK/jdk/bin:$PATH"
java -version 2>&1 | head -1

if [ ! -d nlg/node_modules/npm-groovy-lint ]; then mkdir -p nlg; (cd nlg && npm init -y >/dev/null && npm install --no-audit --no-fund npm-groovy-lint >/dev/null); fi
echo "npm-groovy-lint $(node -p "require('./nlg/node_modules/npm-groovy-lint/package.json').version")"

if [ ! -x gradle/bin/gradle ]; then
  v="$(curl -sSf https://services.gradle.org/versions/current | python3 -I -c 'import json,sys;print(json.load(sys.stdin)["version"])')"
  curl -sSfL -o gradle.zip "https://services.gradle.org/distributions/gradle-$v-bin.zip"
  rm -rf gradle; unzip -q gradle.zip && mv "gradle-$v" gradle && rm gradle.zip
fi
gradle/bin/gradle --version | sed -n 3p

# Spotless: the plugin version is the latest on the plugin portal at install time
if [ ! -f spotless/build.gradle ]; then
  mkdir -p spotless
  sv="$(curl -sSf https://plugins.gradle.org/m2/com/diffplug/spotless/com.diffplug.spotless.gradle.plugin/maven-metadata.xml | sed -n 's|.*<release>\(.*\)</release>.*|\1|p')"
  printf 'rootProject.name = "spotless-eval"\n' > spotless/settings.gradle
  cat > spotless/build.gradle <<EOF
plugins { id 'com.diffplug.spotless' version '$sv' }
repositories { mavenCentral() }
spotless {
  groovyGradle {
    target 'corpus/**/*.gradle'
    greclipse()
    indentWithSpaces(4)
    ignoreErrorForStep('eclipse groovy formatter')
  }
}
EOF
fi
grep -o "version '[^']*'" spotless/build.gradle

# tree-sitter grammar candidates: sources under ts/<name>/
mkdir -p ts
fetch_gh() { # name repo
  [ -d "ts/$1" ] && return
  sha="$(gh api "repos/$2/commits/HEAD" --jq .sha)"
  mkdir -p "ts/$1.tmp"; curl -sSfL "https://codeload.github.com/$2/tar.gz/$sha" | tar -xz -C "ts/$1.tmp" --strip-components=1; echo "$2 $sha" > "ts/$1.tmp/PIN"; mv "ts/$1.tmp" "ts/$1"
}
fetch_crate() { # name crate version
  [ -d "ts/$1" ] && return
  mkdir -p "ts/$1.tmp"; curl -sSfL "https://crates.io/api/v1/crates/$2/$3/download" -A sudsy-evidence | tar -xz -C "ts/$1.tmp" --strip-components=1; echo "crate $2 $3" > "ts/$1.tmp/PIN"; mv "ts/$1.tmp" "ts/$1"
}
fetch_gh murtaza64 murtaza64/tree-sitter-groovy
fetch_gh decodetalkers Decodetalkers/tree-sitter-groovy
fetch_gh loganprice loganprice/tree-sitter-groovy
fetch_crate amaanq tree-sitter-groovy 0.1.2
fetch_crate dekobon dekobon-tree-sitter-groovy 0.6.0
for d in ts/*; do echo "$d $(cat "$d/PIN") parser.c: $(ls "$d"/src/parser.c "$d"/*/src/parser.c 2>/dev/null | head -1)"; done
