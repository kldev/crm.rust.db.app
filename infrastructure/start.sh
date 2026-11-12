#!/usr/bin/env bash
# Local stack. Usage:
#   ./start.sh                      build and start (postgres, db-init, migrate, app)
#   ./start.sh --clean              remove containers and the database volume first
#   ./start.sh --seed               also run the seed (values from .env; override e.g. CRM_SEED_CLIENTS=1000 ./start.sh --seed)
#   ./start.sh --k6 [script.js] [name]  run k6 (default crm-mixed.js), results in perf/results/<name>
#   ./start.sh --down               stop everything
set -euo pipefail
cd "$(dirname "$0")"

if [[ ! -f .env ]]; then
  echo "infrastructure/.env is missing: cp .env-sample .env and set the passwords." >&2
  exit 1
fi

compose() { docker compose --env-file .env "$@"; }

clean=false; seed=false; k6=""; k6_name=""; down=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --clean) clean=true ;;
    --seed) seed=true ;;
    --k6) k6="${2:-crm-mixed.js}"; [[ $# -gt 1 && "$2" != --* ]] && shift
          if [[ $# -gt 1 && "$2" != --* ]]; then k6_name="$2"; shift; fi ;;
    --down) down=true ;;
    -h|--help) sed -n '2,8p' "$0"; exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done

if $down; then compose --profile seed --profile perf down; exit 0; fi
if $clean; then compose --profile seed --profile perf down -v; fi

compose up -d --build --wait postgres app

if $seed; then
  echo "Seeding (this takes several minutes for the default 5M clients / 5M projects)..."
  compose --profile seed run --rm seed
fi

if [[ -n "$k6" ]]; then
  name="${k6_name:-$(date +%F)-${k6%.js}}"
  K6_UID=$(id -u) K6_GID=$(id -g) K6_SCRIPT="$k6" K6_RESULTS_NAME="$name" compose --profile perf run --rm k6
  echo "Results: perf/results/$name"
fi

port=$(grep -E '^CRM_HTTP_PORT=' .env | cut -d= -f2); port=${port:-8088}
echo "API:     http://127.0.0.1:${port}/swagger-ui.html"
