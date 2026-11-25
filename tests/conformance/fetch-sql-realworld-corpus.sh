#!/usr/bin/env bash
# Fetches the pinned real-world SQL corpus into tests/conformance/corpus/sql-real/<dialect>/<repo> (gitignored):
# schemas, migrations, upgrade scripts, DBA scripts and analytics queries of permissively licensed projects
# (Apache-2.0, MIT, PostgreSQL license, CC0), no test suites of SQL tools or parsers. The dialect of a file is the one
# its project documents for it (per-database script folders, `ddl-generic` = ansi), fixed in the lines below before any
# measurement. Files over 200 kB (data dumps) are dropped. Line: `repo sha dialect:tar-pattern...` (dialects: ansi,
# postgres, mysql, tsql); one tarball per repo, files copied per dialect.
set -euo pipefail
set -f  # the patterns are not for the shell
root="$(cd "$(dirname "$0")" && pwd)/corpus/sql-real"
mkdir -p "$root"

fetch_repo() {
  local repo=$1 sha=$2 tmp="$root/.tmp-${1#*/}" spec dialect pat out
  shift 2
  [ -f "$root/.done-${repo#*/}" ] && return
  rm -rf "$tmp"; mkdir -p "$tmp"
  local args=()
  for spec in "$@"; do args+=(--wildcards "${spec#*:}"); done
  curl -sSfL "https://codeload.github.com/$repo/tar.gz/$sha" | tar -xz -C "$tmp" --strip-components=1 "${args[@]}" || true
  for spec in "$@"; do
    dialect=${spec%%:*}; pat=${spec#*:}
    out="$root/$dialect/${repo#*/}"
    mkdir -p "$out"
    (cd "$tmp" && find . -type f -path "*/${pat#\*/}" -size -200000c -print0) | while IFS= read -r -d '' f; do
      mkdir -p "$out/$(dirname "$f")"; cp "$tmp/$f" "$out/$f"
    done
  done
  rm -rf "$tmp"; touch "$root/.done-${repo#*/}"
}

while read -r repo sha specs; do
  [ -z "$repo" ] && continue
  fetch_repo "$repo" "$sha" $specs
done <<'EOT'
apache/hive 42d1cbc33bfaa76ba5e7c28b24add75d10e22d63 postgres:*/metastore-server/src/main/sql/postgres/*.sql mysql:*/metastore-server/src/main/sql/mysql/*.sql tsql:*/metastore-server/src/main/sql/mssql/*.sql
apache/ranger 1792c54b922dee9df44da35136e8fd96311de7c1 postgres:*/security-admin/db/postgres/*.sql mysql:*/security-admin/db/mysql/*.sql tsql:*/security-admin/db/sqlserver/*.sql
apache/dolphinscheduler 390445d11882cb50337d614e2294ff418e48a47b postgres:*/dolphinscheduler-dao/src/main/resources/sql/*postgresql*.sql mysql:*/dolphinscheduler-dao/src/main/resources/sql/*mysql*.sql
kestra-io/kestra 856761fe4b74002150e9d4329c8a512c10ad833f postgres:*/jdbc-postgres/src/main/resources/migrations/*.sql mysql:*/jdbc-mysql/src/main/resources/migrations/*.sql
temporalio/temporal 4197dfdd52eb338c932e64db209e3853fc7a032d postgres:*/schema/postgresql/*.sql mysql:*/schema/mysql/*.sql
openfga/openfga d67ba7b1d2eaf945cffab3a9129ca1a4c7ad5251 postgres:*/assets/migrations/postgres/*.sql mysql:*/assets/migrations/mysql/*.sql
spring-projects/spring-petclinic 500158f732419217507c7656904b8e6aa1bcc0d6 postgres:*/src/main/resources/db/postgres/*.sql mysql:*/src/main/resources/db/mysql/*.sql
spring-projects/spring-batch 8352aa1b511679acf5511706534a1937b9022cae postgres:*/batch/core/schema-*postgresql.sql mysql:*/batch/core/schema-*mysql.sql tsql:*/batch/core/schema-*sqlserver.sql
spring-projects/spring-session 19b331d88cd4d61eaf904061d699d09b4e90eaeb postgres:*/session/jdbc/schema-*postgresql.sql mysql:*/session/jdbc/schema-*mysql.sql tsql:*/session/jdbc/schema-*sqlserver.sql
pgvector/pgvector 3d4967a589d7169d08cf06ef8c3cee1c268b9c13 postgres:*/sql/vector*.sql
devrimgunduz/pagila 6ea53f812254c91a602f61c0ef62beca3fa676b1 postgres:*/pagila-schema*.sql
cmu-db/benchbase 33c00473807ebd49304d114a6d769d2d2b2bbb34 ansi:*/resources/benchmarks/*/ddl-generic.sql postgres:*/resources/benchmarks/*/ddl-postgres.sql mysql:*/resources/benchmarks/*/ddl-mysql.sql tsql:*/resources/benchmarks/*/ddl-sqlserver.sql
apache/datafusion f681868d62847043da8eca4de7e8acc016ff129c ansi:*/benchmarks/queries/q*.sql ansi:*/benchmarks/queries/imdb/*.sql
BrentOzarULTD/SQL-Server-First-Responder-Kit 5d3a6811a44e4aef1115aee75254e930e2e60938 tsql:*/sp_*.sql tsql:*/Install-*.sql
olahallengren/sql-server-maintenance-solution 41617578643e722b790ea4d57ef80f8de38ae747 tsql:*/*.sql
microsoft/tigertoolbox fe47c31ec2f7979a994520d546c7abf0b246c832 tsql:*/*.sql
ktaranov/sqlserver-kit 82643dabd30d08d5981861a1cd1922ae0f842116 tsql:*/Stored_Procedure/*.sql tsql:*/User_Defined_Function/*.sql
EOT
echo "corpus ready: $root"
