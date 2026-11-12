# syntax=docker/dockerfile:1
# One image, four modes (CRM_MODE): app (default), init (roles and database, then exit),
# migrate (db/migration, then exit), seed (test data, then exit).
# The React app (frontend/), built once and served by the Rust binary from /app/static.
FROM node:22-alpine AS web
WORKDIR /web
RUN corepack enable
COPY frontend/package.json frontend/yarn.lock frontend/.yarnrc.yml ./
RUN --mount=type=cache,target=/root/.yarn/berry/cache yarn install --immutable
COPY frontend/ ./
RUN yarn build

FROM rust:1.99.1-slim-trixie AS build
WORKDIR /src
# The SQL (db/) is embedded in the binary (include_dir), so it sits next to server/ as in the repository.
COPY db db
COPY server/Cargo.toml server/Cargo.lock server/
COPY server/assets server/assets
COPY server/src server/src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/server/target \
    cd server && cargo build --release --locked && cp target/release/crm /crm

FROM debian:trixie-slim AS runtime
# HEALTHCHECK calls curl.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 app \
    && useradd --system --uid 10001 --gid app --no-create-home app

WORKDIR /app
COPY --from=build --chown=10001:10001 /crm /app/crm
COPY --from=web --chown=10001:10001 /web/dist /app/static
ENV CRM_STATIC_DIR=/app/static PORT=8080

USER 10001:10001
EXPOSE 8080

# Only the web mode has an HTTP port; init, migrate and seed are one-shot containers.
HEALTHCHECK --interval=10s --timeout=5s --start-period=10s --retries=5 \
    CMD [ "${CRM_MODE:-app}" != "app" ] || curl -fsS http://localhost:${PORT:-8080}/actuator/health/readiness || exit 1

ENTRYPOINT ["/app/crm"]
