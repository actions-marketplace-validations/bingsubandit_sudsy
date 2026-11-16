#!/usr/bin/env bash
# Installs the pinned Temurin JRE for this host below <dir>/temurin-<release>-<os>-<arch> and prints
# the path of its `java`: scripts/lib/jre.sh <dir>. The tarball comes from the Adoptium API and is
# checked against the SHA-256 the API publishes for it. Shared by the parity runners of JVM tools
# (ktlint, google-java-format). Needs curl, tar and python3.
set -euo pipefail
release='jdk-21.0.13+9'
dir=${1:?usage: jre.sh <dir>}
case "$(uname -s)" in Linux) os=linux ;; Darwin) os=mac ;; *) os=windows ;; esac
case "$(uname -m)" in x86_64 | amd64) arch=x64 ;; aarch64 | arm64) arch=aarch64 ;; *) echo "jre.sh: no JRE for $(uname -m)" >&2; exit 1 ;; esac
home=$dir/temurin-${release/+/_}-$os-$arch
java=$home/bin/java
if [ ! -x "$java" ]; then
  mkdir -p "$home"
  enc=${release/+/%2B}
  sum=$(curl -fsSL "https://api.adoptium.net/v3/assets/release_name/eclipse/$enc?architecture=$arch&image_type=jre&os=$os" |
    python3 -c 'import json,sys; print(json.load(sys.stdin)["binaries"][0]["package"]["checksum"])')
  tmp=$home.download
  curl -fsSL -o "$tmp" "https://api.adoptium.net/v3/binary/version/$enc/$os/$arch/jre/hotspot/normal/eclipse"
  echo "$sum  $tmp" | sha256sum -c --status - || { echo "jre.sh: checksum mismatch" >&2; rm -f "$tmp"; exit 1; }
  tar -xzf "$tmp" -C "$home" --strip-components=1
  rm -f "$tmp"
  # macOS tarballs keep the JRE under Contents/Home
  [ -x "$java" ] || cp -R "$home"/Contents/Home/. "$home"/
fi
echo "$java"
