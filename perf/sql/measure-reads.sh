#!/usr/bin/env bash
# Runs measure-reads.sql in the compose postgres container (superuser) and writes <dir>/sql-reads.md.
set -euo pipefail
cd "$(dirname "$0")/../../infrastructure"
out="${1:?usage: measure-reads.sh <results dir>}"
mkdir -p "$out"
db=$(grep -E '^CRM_DB_NAME=' .env | cut -d= -f2 || true); db=${db:-crm}
docker compose --env-file .env exec -T postgres psql -U postgres -d "$db" -v ON_ERROR_STOP=1 -X \
  < ../perf/sql/measure-reads.sql > "$out/sql-reads.md"
echo "written: $out/sql-reads.md"
