#!/usr/bin/env bash
# Fetches the pinned Ruby corpus into tests/conformance/corpus/ruby/<repo> (gitignored): Rails,
# Discourse, RuboCop and Jekyll (`.rb` and `.rake` files).
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs ruby <<'EOT'
rails/rails a5203b3d4d8461550ff8891c28932acab192648c *.rb *.rake
discourse/discourse 47f984080ff7cb495db3cdfb4b792bf3d583dce6 *.rb *.rake
rubocop/rubocop dacb36c343c0d2fd36adb52b462534b7f598df12 *.rb *.rake
jekyll/jekyll 97c3bdc822df9d5bea4c269b29a28de5a5e29d70 *.rb *.rake
EOT
