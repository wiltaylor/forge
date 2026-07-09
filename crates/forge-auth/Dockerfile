# syntax=docker/dockerfile:1
# Build (repo root): docker build -f crates/forge-auth/Dockerfile -t forge-auth .
# Dev build:         docker build -f crates/forge-auth/Dockerfile --build-arg FEATURES=dev-login -t forge-auth:dev .
#
# Or via just: `just auth-docker-build` / `just auth-docker-build-dev`.

# --- stage 1: frontend ------------------------------------------------------
FROM node:22-slim AS frontend
RUN npm install -g pnpm@10
WORKDIR /build
# Whole repo: pnpm --frozen-lockfile needs every workspace package present,
# and turbo resolves the @forge/tokens + @forge/ui dependency graph itself.
COPY . .
RUN pnpm install --frozen-lockfile \
 && pnpm exec turbo build --filter=@forge/auth

# --- stage 2: server --------------------------------------------------------
FROM rust:1-bookworm AS backend
ARG FEATURES=""
WORKDIR /build
# Every cargo workspace member (plus vendor/ for the [patch.crates-io] entry)
# must be present for the workspace to parse; only -p forge-auth is compiled.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY examples/rust-demo ./examples/rust-demo
COPY examples/auth-demo ./examples/auth-demo
COPY vendor ./vendor
COPY --from=frontend /build/apps/auth/dist ./apps/auth/dist
RUN cargo build --release --locked -p forge-auth ${FEATURES:+--features $FEATURES}

# --- stage 3: runtime -------------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -r -u 10001 -d /data forgeauth \
 && mkdir -p /data && chown forgeauth /data
COPY --from=backend /build/target/release/forge-auth /usr/local/bin/forge-auth
USER forgeauth
VOLUME /data
ENV FORGE_HOST=0.0.0.0 \
    FORGE_PORT=8770 \
    DATABASE_URL="sqlite:///data/forge-auth.db?mode=rwc"
EXPOSE 8770
ENTRYPOINT ["/usr/local/bin/forge-auth"]
