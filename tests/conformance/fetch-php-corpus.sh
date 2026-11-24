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
# The tarball leaves out what .gitattributes marks export-ignore of Symfony (src/Symfony/Contracts, Component/Runtime, Bridge/PhpUnit and
# the Bridge folders of Mailer, Messenger, Notifier and Translation), which a checkout has: a sparse checkout of them is
# tests/conformance/php-deps/symfony/monorepo (gitignored). The root composer.json of Symfony takes Contracts and Runtime from path
# repositories there (the manifest of php-deps/symfony names them), and the PHPStan run analyses the folders as part of the corpus.
# Their `/Tests` folders are left out as in the tarball (export-ignore of each component). Bridge/PhpUnit is not part of it: the root requires symfony/phpunit-bridge from Packagist, and the same classes twice (corpus and vendor) crash PHPStan.
mono="$(cd "$(dirname "$0")" && pwd)/php-deps/symfony/monorepo"
if [ ! -d "$mono" ]; then
  rm -rf "$mono.tmp"; mkdir -p "$mono.tmp"
  git -C "$mono.tmp" init -q
  git -C "$mono.tmp" fetch -q --depth 1 --filter=blob:none https://github.com/symfony/symfony.git a9bd44fa9c1aec723ae497a41ead077c1986b4f4
  git -C "$mono.tmp" sparse-checkout set --no-cone /src/Symfony/Contracts /src/Symfony/Component/Runtime \
    /src/Symfony/Component/Mailer/Bridge /src/Symfony/Component/Messenger/Bridge /src/Symfony/Component/Notifier/Bridge /src/Symfony/Component/Translation/Bridge
  git -C "$mono.tmp" checkout -q FETCH_HEAD
  rm -rf "$mono.tmp/.git"
  mv "$mono.tmp" "$mono"
fi
