#!/usr/bin/env bash
# Fetches the pinned SQL corpus into tests/conformance/corpus/sql/<repo> (gitignored): sqlfluff's dialect
# fixtures (ansi, postgres, mysql, tsql; MIT) and PostgreSQL's regression test inputs.
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs sql <<'EOT'
sqlfluff/sqlfluff 0511a32150782ad829206ca41e41770de9de1fcc */test/fixtures/dialects/ansi/*.sql */test/fixtures/dialects/postgres/*.sql */test/fixtures/dialects/mysql/*.sql */test/fixtures/dialects/tsql/*.sql
postgres/postgres 81b2179b492e3af891544813b7e22a42386b34d0 */src/test/regress/sql/*.sql
EOT
