#!/usr/bin/env bash
# Fetches the pinned PHP corpus into tests/conformance/corpus/php/<repo> (gitignored): Laravel,
# Symfony (with its test fixtures), WordPress and Composer (`.php` files). The dependencies of the projects
# are not part of it: `scripts/parity.sh` installs them below the PHPStan prefix (`composer` of its entry in
# scripts/parity_tools.json) from tests/conformance/php-deps/<project>/composer.json, which pins the exact
# versions of the projects without a lock file; Composer's own composer.json and composer.lock come from the
# same pinned tarball, into corpus/php-deps/composer.
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs php <<'EOT'
laravel/framework b65b5e31522b8a3fcf2bcc7497fa6fb0e8f57ca1 *.php
symfony/symfony a9bd44fa9c1aec723ae497a41ead077c1986b4f4 *.php
WordPress/WordPress 6558c0972ee1826c016d6ab3173f36ebaf14de89 *.php
composer/composer 5147ea1a01c124f461090a72bee6ea6de02a67a6 *.php
EOT
deps="$(cd "$(dirname "$0")" && pwd)/corpus/php-deps/composer"
if [ ! -f "$deps/composer.lock" ]; then
  mkdir -p "$deps"
  curl -sSfL "https://codeload.github.com/composer/composer/tar.gz/5147ea1a01c124f461090a72bee6ea6de02a67a6" \
    | tar -xz -C "$deps" --strip-components=1 --wildcards --no-wildcards-match-slash '*/composer.json' '*/composer.lock'
fi
