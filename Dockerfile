FROM rust:1-bookworm AS rust-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY openapi openapi
RUN cargo build --release --bin arc

FROM oven/bun:1
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 make g++ ca-certificates && \
    rm -rf /var/lib/apt/lists/*
RUN bun i -g mintlify

WORKDIR /app

COPY --from=rust-builder /app/target/release/arc /usr/local/bin/arc

COPY package.json bun.lock ./
COPY apps/arc-web/package.json apps/arc-web/
COPY packages/arc-api-client/package.json packages/arc-api-client/
RUN bun install --frozen-lockfile

COPY apps/arc-web apps/arc-web
COPY packages/arc-api-client packages/arc-api-client
COPY docs docs
COPY entrypoint.ts .

ENTRYPOINT ["bun", "run", "entrypoint.ts"]
