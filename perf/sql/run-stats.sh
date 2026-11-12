#!/usr/bin/env bash
# perf/sql/run-stats.sh reset | <results dir>  (see run-stats.sql)
set -euo pipefail
cd "$(dirname "$0")/../../infrastructure"
arg="${1:?usage: run-stats.sh reset | <results dir>}"
db=$(grep -E '^CRM_DB_NAME=' .env | cut -d= -f2 || true); db=${db:-crm}
psql() { docker compose --env-file .env exec -T postgres psql -U postgres -d "$db" -v ON_ERROR_STOP=1 -X "$@"; }
if [[ "$arg" == reset ]]; then
  psql -qc "SELECT pg_stat_statements_reset(); SELECT pg_stat_reset();" >/dev/null
  echo "statistics cleared"
else
  mkdir -p "$arg"
  psql < ../perf/sql/run-stats.sql > "$arg/procedure-stats.txt"
  echo "written: $arg/procedure-stats.txt"
fi
